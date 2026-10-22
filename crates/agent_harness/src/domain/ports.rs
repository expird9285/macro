//! Outbound capabilities required by the harness domain.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::domain::model::PermissionPolicyConfig;
use agent_session::domain::connection::RuntimeAttachment;
use agent_session::domain::model::{AgentMcpServers, AgentSessionId};
use agent_session::domain::ports::AgentConnector;
use bot_id::BotId;

use macro_user_id::user_id::MacroUserIdStr;

use super::error::{HarnessError, Result};
use super::model::{
    AgentRuntimeConfig, AnnouncedMessage, CommandOutcome, HarnessCommand, ProvisionedEgress,
    ReplyPresentation, ResolvedReply, SandboxEgress, SessionAnnouncement, SpawnContainer,
    ToolApprovalChange,
};
use super::notifications::PlannedNotification;

/// The distributed destination for a forwarded command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandTarget {
    /// The replica that owns the session actor.
    Replica(agent_session::domain::model::ReplicaId),
}

/// Forwards commands to the replica currently responsible for execution.
pub trait CommandForwarder: Send + Sync + 'static {
    /// Run `command` at `target`.
    fn forward(
        &self,
        session: AgentSessionId,
        command: HarnessCommand,
        target: CommandTarget,
    ) -> impl Future<Output = Result<CommandOutcome>> + Send;
}

/// A forwarder for deployments with exactly one replica.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoPeers;

impl CommandForwarder for NoPeers {
    async fn forward(
        &self,
        session: AgentSessionId,
        _command: HarnessCommand,
        _target: CommandTarget,
    ) -> Result<CommandOutcome> {
        Err(HarnessError::Disconnected(session))
    }
}

#[cfg(test)]
mod test;

/// Loads facts for the domain to resolve a bot's permission policy.
pub trait PermissionPolicySource: Send + Sync + 'static {
    /// The persona choice and harness limit for `bot` right now.
    fn permission_policy(
        &self,
        bot: BotId,
    ) -> impl Future<Output = anyhow::Result<PermissionPolicyConfig>> + Send;
}

/// Resolves the runtime configuration for a bot that may receive agent
/// session triggers.
pub trait AgentRuntimeDirectory: Send + Sync + 'static {
    /// Return a runtime profile for an in-memory agent or `None`.
    fn runtime_for(
        &self,
        bot_id: BotId,
    ) -> impl Future<Output = Result<Option<AgentRuntimeConfig>>> + Send;
}

/// Authorizes the message origins agent prompts answer into.
pub trait MessagePromptContext: Send + Sync + 'static {
    /// Recheck the actor's posting permission and verify the live message belongs
    /// to exactly this parent and root before provisioning or dispatching work.
    fn authorize_origin(
        &self,
        actor: &MacroUserIdStr<'static>,
        origin: &super::model::AnnounceOrigin,
    ) -> impl Future<Output = Result<()>> + Send;
}

/// Composes an agent prompt from raw markdown and trusted session context.
pub trait AgentPromptComposer: Send + Sync + 'static {
    /// Return the markdown that should be delivered to the agent runtime.
    fn compose(
        &self,
        prompt_markdown: &str,
        instructions: Option<&str>,
        people: Option<&super::model::PromptPeople>,
        trigger: Option<&trigger_context::TriggerContext>,
    ) -> impl Future<Output = Result<String>> + Send;
}

/// Who a prompt names, made able to open the session it is for.
pub trait PromptMentions: Send + Sync + 'static {
    /// The users `prompt_markdown` mentions who can now open `session_id`.
    fn share_with_mentioned<'a>(
        &'a self,
        session_id: AgentSessionId,
        actor: Option<&'a MacroUserIdStr<'static>>,
        prompt_markdown: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<MacroUserIdStr<'static>>>> + Send + 'a>>;
}

impl<Mentions: PromptMentions + ?Sized> PromptMentions for Arc<Mentions> {
    fn share_with_mentioned<'a>(
        &'a self,
        session_id: AgentSessionId,
        actor: Option<&'a MacroUserIdStr<'static>>,
        prompt_markdown: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<MacroUserIdStr<'static>>>> + Send + 'a>> {
        (**self).share_with_mentioned(session_id, actor, prompt_markdown)
    }
}

/// A [`PromptMentions`] that finds nobody and shares with nobody.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoPromptMentions;

impl PromptMentions for NoPromptMentions {
    fn share_with_mentioned<'a>(
        &'a self,
        _session_id: AgentSessionId,
        _actor: Option<&'a MacroUserIdStr<'static>>,
        _prompt_markdown: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<MacroUserIdStr<'static>>>> + Send + 'a>> {
        Box::pin(async { Ok(Vec::new()) })
    }
}

/// Delivers the notifications a lifecycle fact warrants.
pub trait AgentSessionNotifier: Send + Sync + 'static {
    /// Send one notification.
    fn notify(
        &self,
        notification: PlannedNotification,
    ) -> Pin<Box<dyn Future<Output = ()> + Send + '_>>;
}

impl<Notifier: AgentSessionNotifier + ?Sized> AgentSessionNotifier for Arc<Notifier> {
    fn notify(
        &self,
        notification: PlannedNotification,
    ) -> Pin<Box<dyn Future<Output = ()> + Send + '_>> {
        (**self).notify(notification)
    }
}

/// An [`AgentSessionNotifier`] that tells nobody.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoopAgentSessionNotifier;

impl AgentSessionNotifier for NoopAgentSessionNotifier {
    fn notify(
        &self,
        _notification: PlannedNotification,
    ) -> Pin<Box<dyn Future<Output = ()> + Send + '_>> {
        Box::pin(async {})
    }
}

/// Speaks for an agent session in the thread that prompted it.
pub trait SessionAnnouncer: Send + Sync + 'static {
    /// Post the message a turn is answered through, returning what it became.
    fn announce(
        &self,
        announcement: SessionAnnouncement,
    ) -> impl Future<Output = Result<AnnouncedMessage>> + Send;

    /// Replace a chat agent's pending reply with how its turn ended.
    fn resolve(&self, resolution: ResolvedReply) -> impl Future<Output = Result<()>> + Send;

    /// Show one message of a reply as it should read now.
    fn present(
        &self,
        presentation: ReplyPresentation,
    ) -> impl Future<Output = Result<()>> + Send;

    /// Say that the session is typing through its bot, or that it stopped.
    fn typing(
        &self,
        typing: super::model::AgentTypingUpdate,
    ) -> impl Future<Output = Result<()>> + Send;
}

/// Told when a tool call in a session's turn starts or stops waiting for the
/// owner's approval.
pub trait HeldToolCallObserver: Send + Sync + 'static {
    /// `session`'s held tool calls changed.
    fn changed(&self, session: AgentSessionId, change: ToolApprovalChange);
}

/// A [`HeldToolCallObserver`] bound after construction.
#[derive(Default)]
pub struct LateBoundHeldToolCallObserver {
    observer: std::sync::OnceLock<Box<dyn HeldToolCallObserver>>,
}

impl LateBoundHeldToolCallObserver {
    /// An observer awaiting its target.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Attach the real observer.
    pub fn bind(&self, observer: impl HeldToolCallObserver) {
        let _ = self.observer.set(Box::new(observer));
    }
}

impl HeldToolCallObserver for LateBoundHeldToolCallObserver {
    fn changed(&self, session: AgentSessionId, change: ToolApprovalChange) {
        if let Some(observer) = self.observer.get() {
            observer.changed(session, change);
        }
    }
}

impl<T: HeldToolCallObserver + ?Sized> HeldToolCallObserver for Arc<T> {
    fn changed(&self, session: AgentSessionId, change: ToolApprovalChange) {
        (**self).changed(session, change);
    }
}

/// Mints the one secret a sandbox is given, and the config that points it at
/// the egress proxy.
pub trait SandboxEgressProvisioner: Send + Sync + 'static {
    /// The session's MCP servers at addresses reachable by an external runtime.
    fn external_mcp_servers(
        &self,
        egress: &SandboxEgress,
    ) -> Vec<agent_client_protocol::schema::v1::McpServer> {
        egress.acp_servers()
    }

    /// The egress environment for one session.
    fn provision(
        &self,
        session: AgentSessionId,
        owner: &MacroUserIdStr<'static>,
        selection: &AgentMcpServers,
    ) -> impl Future<Output = Result<ProvisionedEgress>> + Send;

    /// The egress environment rebuilt around a token that already exists.
    fn restore(
        &self,
        owner: &MacroUserIdStr<'static>,
        session_token: String,
        selection: &AgentMcpServers,
    ) -> impl Future<Output = Result<SandboxEgress>> + Send;
}

/// Provisions the container transports agent sessions run through.
pub trait ContainerManager: Send + Sync + 'static {
    /// Transport returned by this provider.
    type Transport: AgentConnector;

    /// Boot a new container for a session that has never had one.
    fn spawn(
        &self,
        command: SpawnContainer,
    ) -> impl Future<
        Output = Result<agent_session::domain::connection::RuntimeAttachment<Self::Transport>>,
    > + Send;

    /// Reattach to a session's existing container, starting it if stopped.
    fn resume(
        &self,
        session: AgentSessionId,
    ) -> impl Future<
        Output = Result<agent_session::domain::connection::RuntimeAttachment<Self::Transport>>,
    > + Send;

    /// The raw egress session token the session's container holds.
    fn session_token(
        &self,
        session: AgentSessionId,
    ) -> impl Future<Output = Result<Option<String>>> + Send;

    /// Destroy a session's container for good.
    fn teardown(&self, session: AgentSessionId) -> impl Future<Output = Result<()>> + Send;
}
