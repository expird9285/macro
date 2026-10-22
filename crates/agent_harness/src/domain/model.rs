//! Commands and values used by the harness domain.

use agent_client_protocol::schema::v1::{HttpHeader, McpServer as AcpMcpServer, McpServerHttp};
use agent_egress::domain::model::{
    CUSTOM_MCP_PATH_PREFIX, CustomMcpServerKey, CustomMcpServerListing, McpServerSlug,
};
use agent_fold::domain::model::{StopReason, TurnSignal};
use agent_runtime_protocol::domain::action::{AgentAction, AgentActionId, PromptAttachment};
use agent_session::domain::model::{AgentMcpServers, AgentSessionId, MessageId};
use agent_session::domain::ports::ControlEvent;
use agent_session::domain::session::PermissionPolicy;

#[cfg(test)]
mod test;
use bot_id::BotId;
use macro_user_id::user_id::MacroUserIdStr;
use macro_uuid::Uuid;
use messages::domain::events::MessageEventAttachment;
use trigger_context::TriggerContext;
mod session_origin;
pub use session_origin::{MentionOrigin, SessionOrigin, TaskAssignmentOrigin};

/// How a channel message's attached files are named to an agent.
#[derive(Debug, Clone)]
pub struct StaticFileLinks {
    base_url: String,
}

impl StaticFileLinks {
    /// Channel attachment entity type for an image stored as a static file.
    const STATIC_IMAGE: &str = "static/image";
    /// Channel attachment entity type for a video stored as a static file.
    const STATIC_VIDEO: &str = "static/video";

    /// Links under the static file service at `base_url`.
    #[must_use]
    pub fn new(base_url: impl Into<String>) -> Self {
        let mut base_url = base_url.into();
        while base_url.ends_with('/') {
            base_url.pop();
        }
        Self { base_url }
    }

    /// The prompt attachment for a channel attachment, or `None` for one that
    /// is not a static file.
    #[must_use]
    pub fn prompt_attachment(
        &self,
        attachment: &MessageEventAttachment,
    ) -> Option<PromptAttachment> {
        let (kind, mime_type) = match attachment.entity_type.as_str() {
            Self::STATIC_IMAGE => ("image", "image/*"),
            Self::STATIC_VIDEO => ("video", "video/*"),
            _ => return None,
        };
        let uri = format!("{}/file/{}", self.base_url, attachment.entity_id);
        Some(PromptAttachment::new(uri, kind).mime_type(mime_type))
    }

    /// The prompt attachments for a message's attached files, in order.
    #[must_use]
    pub fn prompt_attachments(
        &self,
        attachments: &[MessageEventAttachment],
    ) -> Vec<PromptAttachment> {
        attachments
            .iter()
            .filter_map(|attachment| self.prompt_attachment(attachment))
            .collect()
    }
}

/// Open a new session for a mention or task assignment.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OpenSession {
    /// The bot selected by the triggering event.
    pub bot_id: BotId,
    /// Runtime configuration resolved for this bot when the trigger arrived.
    pub runtime: AgentRuntimeConfig,
    /// The mention or assignment that requested the session.
    pub origin: SessionOrigin,
}

/// How a bot's sessions get a runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum AgentKind {
    /// The in-process (in-memory) Macro agent bot, served by `agent_inmem`.
    InMemory,
}

impl AgentKind {
    /// The kind of runtime serving `bot`'s sessions.
    #[must_use]
    pub fn of(_bot: BotId) -> Self {
        Self::InMemory
    }

    /// Resolve a database-backed agent's runtime from its persisted harness.
    #[must_use]
    pub fn from_harness(_harness: &str) -> Self {
        Self::InMemory
    }

    /// Resolve a persisted session's runtime without losing fixed system-bot
    /// identities that predate per-agent harness configuration.
    #[must_use]
    pub fn for_session(_bot: BotId, _harness: &str) -> Self {
        Self::InMemory
    }

    /// Fixed harness slug for kinds that map one-to-one onto one.
    #[must_use]
    pub const fn harness_slug(self) -> Option<&'static str> {
        None
    }

    /// Whether a session's instructions travel in its first prompt.
    #[must_use]
    pub const fn folds_instructions(self) -> bool {
        false
    }

    /// Whether this kind's runtime starts on the session row's model by itself.
    #[must_use]
    pub const fn starts_on_session_model(self) -> bool {
        true
    }

    /// Whether a deployment provisions this kind's runtimes itself.
    #[must_use]
    pub fn is_managed(self) -> bool {
        true
    }

    /// Whether this kind's agent is handed the session's whole egress server list.
    #[must_use]
    pub fn takes_egress_mcp_servers(self) -> bool {
        true
    }

    /// Whether this kind's sessions work in a repository.
    #[must_use]
    pub const fn is_coding(self) -> bool {
        false
    }

    /// How this kind's sessions answer permission requests without a registered local harness.
    #[must_use]
    pub fn default_permission_policy(self) -> PermissionPolicy {
        PermissionPolicy::AutoAccept
    }
}

/// Whether a session's turns are announced as a coding agent's.
#[must_use]
pub const fn is_coding_agent(_choice: Option<bool>, _kind: AgentKind) -> bool {
    false
}

/// The persona a session's thread replies speak as, read from its bot at
/// the moment of speaking.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplyPersona {
    /// The bot's display name, what a link to the session is labelled.
    pub name: String,
    /// Whether the turn is announced as a magic chip (coding) or answered
    /// as a reply in the thread (chat).
    pub is_coding: bool,
}

/// Stored facts used by the domain to choose a session permission policy.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum PermissionPolicyConfig {
    /// A fixed system bot with no editable persona configuration.
    Fixed(AgentKind),
    /// An editable persona and its harness operator's limit.
    Persona {
        /// The runtime serving this persona.
        kind: AgentKind,
        /// A registered harness's opt-in. `None` denotes a built-in runtime.
        harness_allows_bypass: Option<bool>,
        /// The agent owner's choice for a local harness; absent means prompt.
        auto_accept_permissions: Option<bool>,
    },
}

impl PermissionPolicyConfig {
    /// Apply local harness consent and agent choice, or the built-in policy.
    #[must_use]
    pub fn resolve(self) -> PermissionPolicy {
        match self {
            Self::Fixed(kind) => kind.default_permission_policy(),
            Self::Persona {
                kind,
                harness_allows_bypass,
                auto_accept_permissions,
            } => match harness_allows_bypass {
                Some(allowed) => resolve_permission_policy(allowed, auto_accept_permissions),
                None => kind.default_permission_policy(),
            },
        }
    }
}

/// Resolve a persona's choice within the harness operator's permission limit.
#[must_use]
pub fn resolve_permission_policy(
    allow_bypass: bool,
    auto_accept: Option<bool>,
) -> PermissionPolicy {
    if allow_bypass && auto_accept == Some(true) {
        PermissionPolicy::AutoAccept
    } else {
        PermissionPolicy::Prompt
    }
}

/// Runtime settings used to open one database-backed or fixed agent.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AgentRuntimeConfig {
    /// Which runtime implementation serves the agent.
    pub kind: AgentKind,
    /// Model stamped onto the new session.
    pub model: String,
    /// Harness slug stamped onto the new session.
    pub harness: String,
    /// Configured agent instructions, snapshotted onto each new session's row.
    pub instructions: String,
    /// Which Pipedream MCP servers the agent's sessions are handed.
    pub mcp_servers: AgentMcpServers,
}

/// Whether a user belongs to the Macro staff domain.
pub(crate) use agent_egress::domain::model::is_macro_staff;

/// Where a prompt came from, when it came from somewhere the session should
/// answer back into.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReplyPlacement {
    /// Answer inside the source discussion, the behavior of existing mentions.
    #[default]
    Thread,
    /// Answer in the channel's main timeline, as an agent conversing there does.
    Timeline,
}

impl ReplyPlacement {
    /// Whether this is the default placement used by existing serialized origins.
    pub fn is_thread(&self) -> bool {
        *self == Self::Thread
    }

    /// Convert a source root to the message API's optional reply target.
    pub fn thread_id(self, root: Uuid) -> Option<Uuid> {
        match self {
            Self::Thread => Some(root),
            Self::Timeline => None,
        }
    }

    /// How a turn answered here is shown.
    pub fn voice_style(self) -> VoiceStyle {
        match self {
            Self::Thread => VoiceStyle::Turn,
            Self::Timeline => VoiceStyle::Segments,
        }
    }
}

/// How an agent's reply is shown where it was asked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoiceStyle {
    /// One message per turn, posted when the turn starts and updated in place.
    Turn,
    /// One message per passage, posted when the passage is finished.
    Segments,
}

/// One message of an agent's reply, as it should read now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplyPresentation {
    /// The session the reply speaks for.
    pub session_id: AgentSessionId,
    /// The bot the session speaks as; the message is posted as it.
    pub bot_id: BotId,
    /// Who prompted the turn; the bot writes on their current capability.
    pub triggered_by: MacroUserIdStr<'static>,
    /// Where the reply is shown.
    pub parent: messages::domain::models::MessageParent,
    /// The thread it is shown in; `None` for a timeline.
    pub thread_id: Option<Uuid>,
    /// The message's id, allocated before it is first posted so posting it
    /// again finds the same message.
    pub message_id: Uuid,
    /// The turn the segments belong to.
    pub turn: agent_fold::domain::model::TurnId,
    /// The segments the message shows, in order.
    pub segments: Vec<agent_fold::domain::model::ProjectedSegment>,
    /// Lead with a link to the session.
    pub link: bool,
    /// Show that the turn is still running after the segments.
    pub pending: bool,
    /// How the turn ended, on the reply's last message once it has.
    pub outcome: Option<ReplyOutcome>,
    /// Whether this update is news: the reply's last message, once the turn
    /// has ended.
    pub notify: bool,
}

/// An agent session typing through the bot that speaks for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentTypingUpdate {
    /// The session whose turn is running.
    pub session_id: AgentSessionId,
    /// The bot the session speaks as.
    pub bot_id: BotId,
    /// Who prompted the turn; the bot types on their current capability.
    pub triggered_by: MacroUserIdStr<'static>,
    /// Where the reply will appear.
    pub parent: messages::domain::models::MessageParent,
    /// The thread it will appear in; `None` for a timeline.
    pub thread_id: Option<Uuid>,
    /// Whether the agent is typing.
    pub active: bool,
    /// What the agent is doing.
    pub phase: agent_fold::domain::model::TurnPhase,
}

/// Source message and placement of one agent response.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AnnounceOrigin {
    /// Ordinary mentions reply in a thread; agent conversations answer in the timeline.
    #[serde(default, skip_serializing_if = "ReplyPlacement::is_thread")]
    pub reply_placement: ReplyPlacement,
    /// Update the existing agent response instead of posting a reply.
    #[serde(default)]
    pub reuse_origin_message: bool,
    /// Channel or document the prompt was posted in.
    pub parent: messages::domain::models::MessageParent,
    /// Thread the announcement replies into.
    pub thread_id: Uuid,
    /// The message that triggered the prompt.
    pub message_id: Uuid,
}

/// Whose access a session runs with, and who sent the prompt being composed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromptPeople {
    /// The session's owner.
    pub owner: MacroUserIdStr<'static>,
    /// The prompt's sender; absent when a bot sent it on nobody's behalf.
    pub sender: Option<MacroUserIdStr<'static>>,
}

/// Do something in a session that already exists.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DeliverAction {
    /// The id the action carries onto the wire, minted when it was accepted.
    pub id: AgentActionId,
    /// What the agent is being asked to do.
    pub action: AgentAction,
    /// The user responsible, absent when nobody in particular is.
    pub actor: Option<MacroUserIdStr<'static>>,
    /// Where to announce this, for prompts that arrived from elsewhere.
    pub announce: Option<AnnounceOrigin>,
    /// Why the agent is being asked, rendered ahead of a prompt.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<TriggerContext>,
}

/// One operation executed by the harness for an agent session.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum HarnessCommand {
    /// Open or continue the durably reserved session of an agent's conversation.
    ConversationMessage(OpenSession),
    /// Persist a nonblocking tool card on the replica owning this session.
    PresentTool {
        /// Authenticated session owner.
        actor: MacroUserIdStr<'static>,
        /// Stable component identity and presentation payload.
        presentation: agent_session::domain::model::SessionToolPresentation,
    },
    /// Open a new session.
    Open(OpenSession),
    /// Act on a session that already exists.
    Deliver(DeliverAction),
    /// Replace a queued prompt's text before it dispatches.
    EditQueued {
        /// The queue entry to edit.
        action_id: AgentActionId,
        /// The new raw prompt text.
        prompt: String,
        /// The user responsible.
        actor: Option<MacroUserIdStr<'static>>,
    },
    /// Remove a queued action before it dispatches.
    RemoveQueued {
        /// The queue entry to remove.
        action_id: AgentActionId,
        /// The user responsible.
        actor: Option<MacroUserIdStr<'static>>,
    },
    /// Run a queued action next.
    SteerQueued {
        /// The queue entry to run next.
        action_id: AgentActionId,
        /// The user responsible.
        actor: Option<MacroUserIdStr<'static>>,
    },
    /// The session's fold reported a turn fact.
    Turn(TurnSignal),
    /// The session's live actor stopped.
    SessionStopped {
        /// Why the actor stopped.
        reason: String,
    },
    /// A tool call in the session's running turn started or stopped waiting
    /// for the owner's approval.
    ToolApproval(ToolApprovalChange),
    /// Release a session's live resources and delete it.
    Delete,
}

/// What executing a command did with it, beyond succeeding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandOutcome {
    /// The command ran to completion.
    Completed,
    /// The action waits in the session's queue for the running turn to end.
    Queued,
}

impl DeliverAction {
    /// A prompt from a user, arriving from a channel that may need answering.
    pub fn prompt(
        action: AgentAction,
        actor: Option<MacroUserIdStr<'static>>,
        announce: Option<AnnounceOrigin>,
    ) -> Self {
        Self {
            id: AgentActionId::mint(),
            action,
            actor,
            announce,
            context: None,
        }
    }

    /// A control request under a caller-visible id.
    pub fn control(event: ControlEvent) -> Self {
        Self {
            id: event.action_id.unwrap_or_else(AgentActionId::mint),
            action: event.action,
            actor: event.actor,
            announce: None,
            context: event.context,
        }
    }
}

/// Facts required to announce one prompt into its originating context.
#[derive(Debug, Clone)]
pub struct SessionAnnouncement {
    /// Preallocated reply id for a durably journaled conversation turn.
    pub reply_message_id: Option<Uuid>,
    /// Whether the answer is a thread reply or a top-level channel message.
    pub reply_placement: ReplyPlacement,
    /// Update the existing agent response instead of posting a reply.
    pub reuse_origin_message: bool,
    /// Agent session represented by the announcement.
    pub session_id: AgentSessionId,
    /// The bot the session runs for; the announcement posts as it.
    pub bot_id: BotId,
    /// Whether the bot is a coding agent.
    pub is_coding: bool,
    /// Channel or document containing the mention that opened the session.
    pub origin_parent: messages::domain::models::MessageParent,
    /// Thread where the announcement should be posted.
    pub origin_thread_id: Uuid,
    /// Channel message targeted by the announcement.
    pub origin_message_id: Uuid,
    /// Folded user message that prompts the anchored agent response.
    pub prompted_message_id: MessageId,
    /// Text of the prompting message, shown in the reply target.
    pub prompted_content: String,
    /// User whose mention triggered the announcement.
    pub triggered_by: MacroUserIdStr<'static>,
}

impl SessionAnnouncement {
    /// Assignments and coding agents link to the session; only a chat mention
    /// starts a pending reply that will copy the answer into the discussion.
    pub(crate) const fn shows_session_link(&self) -> bool {
        self.is_coding || self.reuse_origin_message
    }
}

/// The message an announcement became.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnnouncedMessage {
    /// The posted message id.
    pub message_id: Uuid,
}

/// What a turn's reply in the thread that prompted it should say now.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ReplyOutcome {
    /// The agent answered; its last message, whole.
    Answered(String),
    /// The turn ran to its end without saying anything.
    Empty,
    /// The turn was cancelled before it finished.
    Cancelled,
    /// The runtime refused the prompt or died underneath the turn.
    Failed,
    /// The turn is waiting on a question only the session view can answer.
    NeedsInput {
        /// What the agent is asking, in prose.
        question: String,
    },
    /// A tool call is held until the session's owner approves it.
    AwaitingApproval(HeldToolCall),
    /// The question was answered or withdrawn, or every held tool call was
    /// settled, and the turn is running again.
    Resumed,
}

/// A tool call held for the session owner's approval.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct HeldToolCall {
    /// The approval row; what its settling names.
    pub approval_id: String,
    /// `macro`, or the connected app's slug.
    pub server_slug: String,
    /// What a person calls the server the tool is on.
    pub server_name: String,
    /// The tool called.
    pub tool_name: String,
}

/// How a session's held tool calls changed.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ToolApprovalChange {
    /// A call started waiting for the owner.
    Held(HeldToolCall),
    /// The call was approved, denied, cancelled, or expired.
    Settled {
        /// The approval that settled.
        approval_id: String,
    },
}

impl ReplyOutcome {
    /// The outcome of a turn the fold closed with `stop`, whose last text
    /// part was `last_text`.
    #[must_use]
    pub fn of_turn(stop: &StopReason, last_text: Option<String>) -> Self {
        match last_text.filter(|text| !text.trim().is_empty()) {
            Some(text) => Self::Answered(text),
            None => match stop {
                StopReason::Cancelled => Self::Cancelled,
                StopReason::Failed { .. } => Self::Failed,
                StopReason::EndTurn
                | StopReason::MaxTokens
                | StopReason::MaxTurnRequests
                | StopReason::Refusal
                | StopReason::Other { .. } => Self::Empty,
            },
        }
    }
}

/// Facts required to replace a turn's pending reply with what it should say.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedReply {
    /// The session whose turn the reply speaks for; what the reply links to.
    pub session_id: AgentSessionId,
    /// The bot the session runs for; the patch is made as it.
    pub bot_id: BotId,
    /// Whether the session's bot is a coding agent.
    pub is_coding: bool,
    /// The pending reply posted when the turn was announced.
    pub message_id: Uuid,
    /// Channel or document the mention was posted in.
    pub origin_parent: messages::domain::models::MessageParent,
    /// Who prompted the turn.
    pub triggered_by: MacroUserIdStr<'static>,
    /// What the reply should say now.
    pub outcome: ReplyOutcome,
    /// The turn the reply answers.
    pub turn: agent_fold::domain::model::TurnId,
    /// The reply's segments.
    pub segments: Vec<agent_fold::domain::model::ProjectedSegment>,
}

/// Values required to provision a new session container.
#[derive(Debug, Clone)]
pub struct SpawnContainer {
    /// Session that will own the container transport.
    pub session_id: AgentSessionId,
    /// Which provider serves this session.
    pub kind: AgentKind,
    /// How the sandbox reaches anything outside itself.
    pub egress: SandboxEgress,
}

/// Everything a sandbox needs to make an authenticated outbound call.
#[derive(Clone)]
pub struct SandboxEgress {
    /// Base URL of the egress proxy, as the sandbox should dial it.
    pub base_url: String,
    /// The session token, presented on every proxied call.
    pub session_token: String,
    /// The MCP servers to advertise.
    pub mcp_servers: Vec<McpServerSlug>,
    /// The owner's own custom MCP servers.
    pub custom_servers: Vec<CustomMcpServerListing>,
}

/// Where the sandbox finds the egress proxy.
pub const EGRESS_URL_VARIABLE: &str = "MACRO_EGRESS_URL";

/// The session token the sandbox presents on every proxied call.
pub const SESSION_TOKEN_VARIABLE: &str = "MACRO_SESSION_TOKEN";

/// The name every session's server list gives Macro's own MCP server.
pub const MACRO_MCP_NAME: &str = "macro";

/// Harness-owned session tools, separate from the workspace MCP catalog.
pub const INTERNAL_MCP_NAME: &str = "macro_internal";

impl SandboxEgress {
    /// Where the proxy serves `slug`.
    pub fn mcp_url(&self, slug: &McpServerSlug) -> String {
        format!("{}/mcp/{slug}", self.base_url)
    }

    /// Where the proxy serves Macro's own MCP server.
    pub fn macro_mcp_url(&self) -> String {
        format!("{}/mcp-macro", self.base_url)
    }

    /// Where the proxy serves the owner's custom server behind `key`.
    pub fn custom_mcp_url(&self, key: &CustomMcpServerKey) -> String {
        format!("{}{CUSTOM_MCP_PATH_PREFIX}{key}", self.base_url)
    }

    /// The `Authorization` value presented on every proxied call.
    pub fn authorization_header(&self) -> String {
        format!("Bearer {}", self.session_token)
    }

    /// The session token [`Self::acp_servers`] stamped on `servers`.
    pub fn session_token_in(servers: &[AcpMcpServer]) -> Option<String> {
        servers.iter().find_map(|server| match server {
            AcpMcpServer::Http(server) if server.name == INTERNAL_MCP_NAME => server
                .headers
                .iter()
                .find(|header| header.name.eq_ignore_ascii_case("authorization"))
                .and_then(|header| header.value.strip_prefix("Bearer "))
                .map(str::to_owned),
            _ => None,
        })
    }

    /// The sandbox environment this becomes.
    pub fn environment(&self) -> impl IntoIterator<Item = (String, String)> {
        [
            (EGRESS_URL_VARIABLE.to_owned(), self.base_url.clone()),
            (
                SESSION_TOKEN_VARIABLE.to_owned(),
                self.session_token.clone(),
            ),
        ]
    }

    /// The workspace and internal MCP servers, followed by the owner's apps
    /// and custom servers.
    pub fn server_entries(&self) -> impl Iterator<Item = (String, String)> + '_ {
        let mut entries: Vec<(String, String)> = vec![
            (MACRO_MCP_NAME.to_owned(), self.macro_mcp_url()),
            (
                INTERNAL_MCP_NAME.to_owned(),
                format!("{}/mcp/internal", self.base_url),
            ),
        ];
        entries.extend(
            self.mcp_servers
                .iter()
                .map(|slug| (slug.as_str().to_owned(), self.mcp_url(slug))),
        );
        for listing in &self.custom_servers {
            let name = unique_server_name(&listing.name, &listing.key, &entries);
            entries.push((name, self.custom_mcp_url(&listing.key)));
        }
        entries.into_iter()
    }

    /// Session-scoped internal tools.
    pub fn internal_mcp_server(&self) -> AcpMcpServer {
        AcpMcpServer::Http(
            McpServerHttp::new(INTERNAL_MCP_NAME, format!("{}/mcp/internal", self.base_url))
                .headers(vec![HttpHeader::new(
                    "Authorization",
                    self.authorization_header(),
                )]),
        )
    }

    /// The MCP servers an ACP agent is handed.
    pub fn acp_servers(&self) -> Vec<AcpMcpServer> {
        self.server_entries()
            .map(|(name, url)| {
                AcpMcpServer::Http(McpServerHttp::new(name, url).headers(vec![HttpHeader::new(
                    "Authorization",
                    self.authorization_header(),
                )]))
            })
            .collect()
    }
}

/// A minted egress environment and the hash that has to be stored for it to work.
#[derive(Debug, Clone)]
pub struct ProvisionedEgress {
    /// SHA-256 hex of the session token, for the session row.
    pub session_token_hash: String,
    /// The environment the sandbox is spawned with, carrying the raw token.
    pub sandbox: SandboxEgress,
}

fn unique_server_name(name: &str, key: &CustomMcpServerKey, taken: &[(String, String)]) -> String {
    const DEFAULT_NAME: &str = "custom";
    const KEY_SUFFIX_LEN: usize = 8;

    let mut sanitized: String = name
        .trim()
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '_' | '-') {
                character
            } else {
                '_'
            }
        })
        .collect();
    if sanitized.is_empty() {
        sanitized = DEFAULT_NAME.to_owned();
    }
    let is_taken = |candidate: &str| taken.iter().any(|(existing, _)| existing == candidate);
    if !is_taken(&sanitized) {
        return sanitized;
    }
    let key = key.as_str();
    let suffixed = format!("{sanitized}_{}", &key[..KEY_SUFFIX_LEN.min(key.len())]);
    if !is_taken(&suffixed) {
        return suffixed;
    }
    format!("{sanitized}_{key}")
}

impl std::fmt::Debug for SandboxEgress {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SandboxEgress")
            .field("base_url", &self.base_url)
            .field("session_token", &"[REDACTED]")
            .field("mcp_servers", &self.mcp_servers)
            .field("custom_servers", &self.custom_servers)
            .finish()
    }
}

/// Session-row values that remain deployment configuration for now.
#[derive(Debug, Clone)]
pub struct SessionDefaults {
    /// The bot managed sessions run as.
    pub bot_id: BotId,
    /// Model slug.
    pub model: String,
    /// Harness slug.
    pub harness: String,
}

/// Session defaults for every bot a deployment answers for.
#[derive(Debug, Clone)]
pub struct HarnessDefaults {
    default: SessionDefaults,
    per_bot: Vec<(BotId, SessionDefaults)>,
    managed_bot: Option<BotId>,
}

impl HarnessDefaults {
    /// Defaults every bot shares until one is given its own.
    #[must_use]
    pub fn new(default: SessionDefaults) -> Self {
        Self {
            default,
            per_bot: Vec::new(),
            managed_bot: None,
        }
    }

    /// Stamp `bot`'s sessions with `defaults` instead of the shared ones.
    #[must_use]
    pub fn with_bot(mut self, bot: BotId, defaults: SessionDefaults) -> Self {
        self.per_bot.push((bot, defaults));
        self
    }

    /// Open managed sessions as `bot` instead of the deployment's own.
    #[must_use]
    pub fn with_managed_bot(mut self, bot: BotId) -> Self {
        self.managed_bot = Some(bot);
        self
    }

    /// The defaults a session opens with when no particular bot is named.
    #[must_use]
    pub fn managed(&self) -> &SessionDefaults {
        match self.managed_bot {
            Some(bot) => self.for_bot(bot),
            None => &self.default,
        }
    }

    /// The defaults `bot`'s sessions are stamped with.
    #[must_use]
    pub fn for_bot(&self, bot: BotId) -> &SessionDefaults {
        self.per_bot
            .iter()
            .find(|(candidate, _)| *candidate == bot)
            .map_or(&self.default, |(_, defaults)| defaults)
    }
}

impl From<SessionDefaults> for HarnessDefaults {
    fn from(default: SessionDefaults) -> Self {
        Self::new(default)
    }
}
