//! Routes container provisioning for the in-process agent runtime.

use agent_egress::domain::model::SessionToken;
use agent_harness::domain::error::{HarnessError, Result};
use agent_harness::domain::model::SpawnContainer;
use agent_harness::domain::ports::ContainerManager;
use agent_inmem::domain::engine::AgentIdentity;
use agent_inmem::outbound::manager::{InMemAgentManager, SessionFacts};
use agent_runtime_protocol::domain::connection::ServerChannel;
use agent_runtime_protocol::domain::ports::{Transport, TransportError, TransportSender};
use agent_runtime_protocol::domain::schema::v0::{ToRuntimeMessage, ToServerMessage};
use agent_session::domain::error::AgentSessionError;
use agent_session::domain::model::{AgentSessionId, SessionManagement};
use agent_session::domain::service::AgentSessionService;

#[cfg(test)]
mod test;

/// A session transport from the in-process runtime.
pub enum RoutedTransport {
    /// An in-process session's channel.
    InMem(ServerChannel),
}

/// The sending half of a [`RoutedTransport`].
pub enum RoutedSender {
    /// Sends into the in-process agent.
    InMem(tokio::sync::mpsc::UnboundedSender<ToRuntimeMessage>),
}

impl Transport<ToRuntimeMessage, ToServerMessage> for RoutedTransport {
    type Sender = RoutedSender;
    type Receiver = tokio::sync::mpsc::UnboundedReceiver<ToServerMessage>;

    fn split(self) -> (Self::Sender, Self::Receiver) {
        match self {
            Self::InMem(channel) => {
                let (sender, receiver) = channel.split();
                (RoutedSender::InMem(sender), receiver)
            }
        }
    }
}

impl TransportSender<ToRuntimeMessage> for RoutedSender {
    async fn send(&self, message: ToRuntimeMessage) -> std::result::Result<(), TransportError> {
        match self {
            Self::InMem(sender) => TransportSender::send(sender, message).await,
        }
    }
}

/// The shared in-memory runtime used by every in-memory agent session.
pub struct InMemRuntime {
    /// In-memory session provisioner.
    pub manager: InMemAgentManager,
}

/// The deployment's [`ContainerManager`]: in-process for the in-memory bot
/// when one is configured.
pub struct RoutedContainers<Sessions> {
    inmem: Option<InMemRuntime>,
    sessions: Sessions,
}

#[derive(Debug)]
enum Route {
    InMem(Box<SessionFacts>),
}

impl<Sessions> RoutedContainers<Sessions>
where
    Sessions: AgentSessionService,
{
    /// Route by bot: `sessions` is where a session's bot is looked up.
    pub fn new(inmem: Option<InMemRuntime>, sessions: Sessions) -> Self {
        Self { inmem, sessions }
    }

    /// Which runtime serves `session`. The row exists before any container
    /// operation - `open` creates it first - so the lookup is authoritative.
    async fn route(&self, session: AgentSessionId) -> Result<Route> {
        let row = self
            .sessions
            .get_session(session)
            .await
            .map_err(HarnessError::Session)?;
        if self.inmem.is_some() {
            let bot = self
                .sessions
                .session_bot(row.bot_id)
                .await
                .map_err(HarnessError::Session)?;
            return Ok(Route::InMem(Box::new(SessionFacts {
                id: session,
                owner: row.owner_id,
                model: row.model,
                identity: Some(AgentIdentity {
                    bot: row.bot_id,
                    handle: bot_id::mention_handle(row.bot_id, &bot.handle).to_owned(),
                    name: bot.name,
                }),
                instructions: row.instructions,
                acp_session_id: row.acp_session_id,
            })));
        }
        Err(HarnessError::Container(format!(
            "session {session} belongs to the in-process bot, which this deployment does not serve"
        )))
    }

    fn inmem(&self) -> &InMemRuntime {
        self.inmem
            .as_ref()
            .expect("an in-memory route exists only when the runtime is configured")
    }
}

impl<Sessions> ContainerManager for RoutedContainers<Sessions>
where
    Sessions: AgentSessionService,
{
    type Transport = RoutedTransport;

    async fn spawn(
        &self,
        command: SpawnContainer,
    ) -> Result<agent_session::domain::connection::RuntimeAttachment<Self::Transport>> {
        let Route::InMem(facts) = self.route(command.session_id).await?;
        Ok(agent_session::domain::connection::RuntimeAttachment::solo(
            RoutedTransport::InMem(
                self.inmem()
                    .manager
                    .attach(*facts, Some(command.egress.session_token))
                    .await?,
            ),
        ))
    }

    /// An in-process session's raw token lives only in the process that was
    /// handed it, and the row keeps only its hash. Another replica may have
    /// resumed the session since, rotating the hash past any token this
    /// process still holds, so every resume rotates the credential. A session
    /// another live replica manages is refused before rotating: its claim
    /// would refuse this attach anyway, and the rotation would revoke the
    /// credential its live agent holds.
    async fn resume(
        &self,
        session: AgentSessionId,
    ) -> Result<agent_session::domain::connection::RuntimeAttachment<Self::Transport>> {
        let Route::InMem(facts) = self.route(session).await?;
        let management = self
            .sessions
            .management(session)
            .await
            .map_err(HarnessError::Session)?;
        if let SessionManagement::Peer(manager) = management {
            tracing::info!(
                %session,
                peer = %manager.replica,
                "not rotating the credential of a session another replica manages"
            );
            return Err(HarnessError::Session(AgentSessionError::ManagedElsewhere(
                session,
            )));
        }
        let token = SessionToken::mint();
        self.sessions
            .set_egress_token_hash(session, &token.hash())
            .await
            .map_err(HarnessError::Session)?;
        Ok(agent_session::domain::connection::RuntimeAttachment::solo(
            RoutedTransport::InMem(
                self.inmem()
                    .manager
                    .attach(*facts, Some(token.as_str().to_owned()))
                    .await?,
            ),
        ))
    }

    /// An in-process session has no container environment to read a token
    /// back from, so the manager remembers the one spawn or [`Self::resume`]
    /// handed it.
    async fn session_token(&self, session: AgentSessionId) -> Result<Option<String>> {
        let Route::InMem(_) = self.route(session).await?;
        Ok(self.inmem().manager.session_token(session))
    }

    async fn teardown(&self, session: AgentSessionId) -> Result<()> {
        let Route::InMem(_) = self.route(session).await?;
        self.inmem().manager.teardown(session);
        Ok(())
    }
}
