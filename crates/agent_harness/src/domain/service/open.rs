//! Opening sessions: from a channel mention, from the create menu, and for an
//! external runtime that dials in. Each creates the row, provisions egress
//! where there is a sandbox to give it to, and attaches the runtime.

use crate::domain::model::{ReplyPlacement, SpawnContainer};
use agent_session::domain::model::session_owner_user;
use model_owner::Owner;

use super::*;

impl<
    Sessions,
    Containers,
    Announcer,
    PromptContext,
    PromptComposer,
    Egress,
    Lifecycle,
    Mentions,
    Notifier,
> agent_session::domain::ports::SessionOpener
    for AgentHarnessService<
        Sessions,
        Containers,
        Announcer,
        PromptContext,
        PromptComposer,
        Egress,
        Lifecycle,
        Mentions,
        Notifier,
    >
where
    Sessions: AgentSessionService,
    Containers: ContainerManager,
    Announcer: SessionAnnouncer,
    PromptContext: MessagePromptContext,
    PromptComposer: AgentPromptComposer,
    Egress: SandboxEgressProvisioner,
    Lifecycle: AgentSessionLifecyclePublisher,
    Mentions: PromptMentions,
    Notifier: AgentSessionNotifier,
{
    async fn open_external_session(
        &self,
        _request: agent_session::domain::ports::OpenExternalAgentSession,
    ) -> agent_session::domain::error::Result<AgentSession> {
        Err(AgentSessionError::Forbidden)
    }

    /// Provision the selected managed persona's runtime, open a session on it,
    /// and deliver the first prompt if one came with the request.
    async fn open_managed_session(
        &self,
        request: agent_session::domain::ports::OpenManagedSession,
    ) -> agent_session::domain::error::Result<AgentSession> {
        self.open_managed_session_inner(request, false).await
    }

    async fn warm_session(
        &self,
        owner: Owner,
        id: AgentSessionId,
    ) -> agent_session::domain::error::Result<Option<AgentSession>> {
        self.prepare_warm(owner, id).await
    }

    async fn find_thread_session(
        &self,
        thread_id: macro_uuid::Uuid,
        bot_id: BotId,
    ) -> agent_session::domain::error::Result<Option<AgentSessionId>> {
        match self
            .inner
            .sessions
            .find_for_thread(Some(thread_id), Some(bot_id))
            .await?
        {
            agent_session::domain::model::ThreadSession::CreatedFromThread(session) => {
                Ok(Some(session.id))
            }
            agent_session::domain::model::ThreadSession::None => Ok(None),
        }
    }
}

impl<
    Sessions,
    Containers,
    Announcer,
    PromptContext,
    PromptComposer,
    Egress,
    Lifecycle,
    Mentions,
    Notifier,
>
    AgentHarnessInner<
        Sessions,
        Containers,
        Announcer,
        PromptContext,
        PromptComposer,
        Egress,
        Lifecycle,
        Mentions,
        Notifier,
    >
where
    Sessions: AgentSessionService,
    Containers: ContainerManager,
    Announcer: SessionAnnouncer,
    PromptContext: MessagePromptContext,
    PromptComposer: AgentPromptComposer,
    Egress: SandboxEgressProvisioner,
    Lifecycle: AgentSessionLifecyclePublisher,
    Mentions: PromptMentions,
    Notifier: AgentSessionNotifier,
{
    #[tracing::instrument(err, skip(self, command), fields(
        %session_id,
        bot_id = %command.bot_id,
        message_id = tracing::field::Empty,
        parent = tracing::field::Empty,
        thread_id = tracing::field::Empty,
        agent.trigger.kind = command.origin.kind(),
        agent.session.id = tracing::field::Empty,
    ))]
    pub(super) async fn open(
        &self,
        session_id: AgentSessionId,
        command: OpenSession,
    ) -> Result<()> {
        let OpenSession {
            bot_id,
            runtime,
            origin,
        } = command;
        let actor = origin.actor().clone();
        let announcement = origin.announcement();
        let span = tracing::Span::current();
        span.record("agent.session.id", tracing::field::display(session_id));
        span.record(
            "message_id",
            tracing::field::display(announcement.message_id),
        );
        span.record("parent", tracing::field::debug(&announcement.parent));
        span.record("thread_id", tracing::field::display(announcement.thread_id));
        // Recheck access after the triggering event: a user removed from the
        // parent since mentioning or assigning the agent opens nothing.
        self.prompt_context
            .authorize_origin(&actor, &announcement)
            .await?;

        let runtime = if announcement.reply_placement
            == crate::domain::model::ReplyPlacement::Timeline
            && let Some(store) = &self.conversation_turns
        {
            if let Some(settings) = store.settings(session_id).await? {
                settings.runtime
            } else {
                let permissions = self
                    .permission_policies
                    .permission_policy(bot_id)
                    .await
                    .map_err(AgentSessionError::from)?;
                store
                    .pin_settings(
                        session_id,
                        crate::domain::conversation_turns::ConversationSettings {
                            runtime,
                            permissions,
                        },
                    )
                    .await?
                    .runtime
            }
        } else {
            runtime
        };

        self.admit_open(
            bot_id,
            runtime.kind.harness_slug().unwrap_or(&runtime.harness),
            &actor,
        )
        .await?;

        let sandbox_size = self.sessions.user_sandbox_size(&actor).await?;
        let instructions = origin.session_instructions(&runtime.instructions);

        // Provisioned before the session exists, because the row is what makes
        // the token mean anything: it carries the hash the proxy recognises.
        let egress = self
            .egress
            .provision(session_id, &actor, &runtime.mcp_servers)
            .await?;

        let session = self
            .sessions
            .create_session(CreateAgentSessionParams {
                purpose: if announcement.reply_placement == ReplyPlacement::Timeline {
                    agent_session::domain::model::SessionPurpose::AgentDm
                } else {
                    agent_session::domain::model::SessionPurpose::Standalone
                },
                repo_branch: None,
                id: session_id,
                owner_id: Owner::User(actor.clone()),
                bot_id,
                thread_id: announcement
                    .reply_placement
                    .thread_id(announcement.thread_id),
                originating_message_id: Some(announcement.message_id),
                model: runtime.model.clone(),
                harness: runtime
                    .kind
                    .harness_slug()
                    .unwrap_or(&runtime.harness)
                    .to_owned(),
                repo_url: None,
                workspace: agent_session::MANAGED_CONTAINER_WORKSPACE.to_owned(),
                sandbox_size,
                instructions,
                mcp_servers: runtime.mcp_servers.clone(),
                egress_token_hash: Some(egress.session_token_hash),
            })
            .await?;
        self.publish_opened(&session).await;

        if runtime.kind.is_managed() {
            let mcp_servers = if runtime.kind.takes_egress_mcp_servers() {
                egress.sandbox.acp_servers()
            } else {
                Vec::new()
            };
            let container = match self
                .containers
                .spawn(SpawnContainer {
                    session_id,
                    kind: runtime.kind,
                    egress: egress.sandbox,
                })
                .await
            {
                Ok(container) => container,
                Err(error) => {
                    let _ = self
                        .sessions
                        .mark_disconnected(session_id)
                        .await
                        .inspect_err(|status_error| {
                            tracing::error!(
                                error = ?status_error,
                                %session_id,
                                "failed to mark an unprovisioned session disconnected"
                            );
                        });
                    return Err(error);
                }
            };
            let permission_policy = self
                .permission_policy_for_session(session_id, bot_id)
                .await?;
            self.sessions
                .attach_session(
                    session_id,
                    container
                        .mcp_servers(mcp_servers)
                        .permission_policy(permission_policy),
                )
                .await?;
        }

        let conversation_action =
            if announcement.reply_placement == crate::domain::model::ReplyPlacement::Timeline {
                match &self.conversation_turns {
                    Some(store) => store
                        .get(announcement.message_id, bot_id)
                        .await?
                        .map(|turn| turn.action_id),
                    None => None,
                }
            } else {
                None
            };
        self.enqueue_then_dispatch(
            session_id,
            DeliverAction {
                id: if announcement.reply_placement
                    == crate::domain::model::ReplyPlacement::Timeline
                {
                    conversation_action
                        .unwrap_or_else(|| AgentActionId::from_uuid(announcement.message_id))
                } else {
                    AgentActionId::mint()
                },
                context: origin.context().cloned(),
                action: origin.into_action(),
                actor: Some(actor),
                announce: Some(announcement),
            },
        )
        .await?;
        Ok(())
    }
}
