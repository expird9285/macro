//! Harness domain errors.

use agent_runtime_protocol::domain::action::ActionError;
use agent_runtime_protocol::domain::ports::TransportError;
use agent_session::domain::error::AgentSessionError;
use agent_session::domain::model::AgentSessionId;

/// A `Result` whose error is a [`HarnessError`].
pub type Result<T, E = HarnessError> = std::result::Result<T, E>;

/// A failure while provisioning or running an agent session.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum HarnessError {
    /// New Macro-funded work was refused or could not be validated.
    #[error(transparent)]
    Admission(#[from] ai_billing::AiAdmissionError),
    /// A container could not be spawned or reattached.
    #[error("container unavailable: {0}")]
    Container(String),
    /// The agent would not open an ACP session.
    #[error("acp handshake failed: {0}")]
    Handshake(String),
    /// The session has no container to talk to any more.
    #[error("agent session {0} is no longer connected")]
    Disconnected(AgentSessionId),
    /// The session's transport failed.
    #[error(transparent)]
    Transport(#[from] TransportError),
    /// An action could not be expressed as ACP.
    #[error(transparent)]
    Action(#[from] ActionError),
    /// Building or reading an ACP message failed.
    #[error(transparent)]
    Acp(#[from] agent_client_protocol::Error),
    /// Reading or writing the session's persistent state failed.
    #[error(transparent)]
    Session(#[from] AgentSessionError),
    /// A session command worker stopped before reporting its result.
    #[error("agent session {0} command worker stopped")]
    CommandWorkerStopped(AgentSessionId),
    /// The sandbox's egress environment could not be prepared: no signing
    /// key, no repository, or the owner's connected servers could not be read.
    #[error("failed to provision sandbox egress: {0}")]
    Egress(rootcause::Report),
    /// The session link could not be posted back to the mention's thread.
    #[error("failed to announce the agent session: {0}")]
    Announce(rootcause::Report),
    /// Required context for a channel-originated prompt could not be loaded.
    #[error("failed to load channel prompt context: {0}")]
    PromptContext(rootcause::Report),
    /// The message a prompt answers is gone, was moved, or is no longer one
    /// its sender may write beside. Unlike [`Self::PromptContext`], asking
    /// again cannot succeed.
    #[error("the message an agent prompt answers is no longer valid: {0}")]
    InvalidOrigin(rootcause::Report),
    /// A prompt could not be composed for the agent runtime.
    #[error("failed to compose agent prompt: {0}")]
    PromptComposition(rootcause::Report),
    /// Forwarding a command to the session's managing replica failed.
    #[error("failed to forward an agent session command: {0}")]
    Forward(rootcause::Report),
    /// A bot's persisted agent runtime configuration could not be loaded.
    #[error("failed to resolve agent runtime configuration: {0}")]
    RuntimeDirectory(rootcause::Report),
    /// Who a prompt mentions could not be resolved.
    #[error("failed to resolve prompt mentions: {0}")]
    Mentions(rootcause::Report),
}
