#![deny(missing_docs)]
//! This crate contains all filters for various item types to be used in soup/search.

use non_empty::IsEmpty;
pub use notification_state::NotificationState;
use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};

pub mod ast;

/// Fields that can be searched on in search queries
#[derive(Serialize, Deserialize, Debug, Copy, Clone, EnumString, Display, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(utoipa::ToSchema, schemars::JsonSchema))]
pub enum SearchOn {
    /// Search on the name/title field only
    Name,
    /// Search on the content field only (default)
    #[default]
    Content,
    /// Search on both name and content fields
    NameContent,
}

/// Notification-level filters that apply to an entity type.
#[derive(Debug, Serialize, Deserialize, Default, PartialEq, Clone)]
#[cfg_attr(feature = "schema", derive(utoipa::ToSchema, schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct NotificationFilters {
    /// Include entities with a non-deleted notification in any of these exact states.
    /// Empty means no notification restriction. Active means `[unseen, seen]`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub states: Vec<NotificationState>,
}

impl NotificationFilters {
    /// Whether no notification constraint is requested.
    pub fn is_empty(&self) -> bool {
        self.states.is_empty()
    }

    pub(crate) fn into_unique_states(self) -> Vec<NotificationState> {
        let mut unique = Vec::with_capacity(3);
        for state in self.states {
            if !unique.contains(&state) {
                unique.push(state);
            }
        }
        unique
    }
}

impl IsEmpty for NotificationFilters {
    fn is_empty(&self) -> bool {
        self.states.is_empty()
    }
}

/// Task-only filters nested under document filters.
#[derive(Debug, Serialize, Deserialize, Default, PartialEq, Clone)]
#[cfg_attr(feature = "schema", derive(utoipa::ToSchema, schemars::JsonSchema))]
pub struct TaskFilters {
    /// Include tasks that are created by me, assigned to me, and not completed,
    /// even when they do not match other document filters.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub include_cbm_atm_nc: Option<bool>,
}

impl IsEmpty for TaskFilters {
    fn is_empty(&self) -> bool {
        // false is equivalent to "disabled" and should not affect filtering.
        self.include_cbm_atm_nc != Some(true)
    }
}

/// The document filters used to filter down what documents you search over.
#[derive(Debug, Serialize, Deserialize, Default, PartialEq, Clone)]
#[cfg_attr(feature = "schema", derive(utoipa::ToSchema, schemars::JsonSchema))]
pub struct DocumentFilters {
    /// Document file types to search. Examples: ['pdf'], ['md', 'txt']. Empty to search all file types.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub file_types: Vec<String>,

    /// Document ids to search over. Examples: ['doc1'], ['doc1', 'doc2']. Empty to search all accessible documents.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub document_ids: Vec<String>,

    /// A list of project ids to search within. Examples: ['project1'].
    /// filtering. Empty to ignore project filtering.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub project_ids: Vec<String>,

    /// Filter by document owner principal — a user ('macro|user1@user.com'), a bot
    /// ('bot|<uuid>'), or a team (a bare hyphenated uuid). Examples:
    /// ['macro|user1@user.com'], ['macro|user1@user.com', 'bot|0199...']. Empty to
    /// search all owners.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub owners: Vec<String>,

    /// Filter by document importance. None to ignore, true to pass through (no clause), false to short-circuit and return nothing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub importance: Option<bool>,

    /// Filter by document notification state.
    #[serde(default, skip_serializing_if = "NotificationFilters::is_empty")]
    pub notification_filters: NotificationFilters,

    /// Task-specific filters that only apply to task subtype documents.
    #[serde(default, skip_serializing_if = "TaskFilters::is_empty")]
    pub task_filters: TaskFilters,

    /// Filter by document sub type. Examples: ['task']. Empty to search all sub types.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sub_types: Vec<String>,
}

impl IsEmpty for DocumentFilters {
    fn is_empty(&self) -> bool {
        let DocumentFilters {
            file_types,
            document_ids,
            project_ids,
            owners,
            importance,
            notification_filters,
            task_filters,
            sub_types,
        } = self;
        file_types.is_empty()
            && document_ids.is_empty()
            && project_ids.is_empty()
            && owners.is_empty()
            && importance.is_none()
            && notification_filters.is_empty()
            && task_filters.is_empty()
            && sub_types.is_empty()
    }
}

/// The chat filters used to filter down what chats you search over.
#[derive(Debug, Serialize, Deserialize, Default, PartialEq, Clone)]
#[cfg_attr(feature = "schema", derive(utoipa::ToSchema, schemars::JsonSchema))]
pub struct ChatFilters {
    /// Chat message roles to search. Examples: ['user'], ['assistant']. Empty to search all roles.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub role: Vec<String>,

    /// Chat ids to search over. Examples: ['chat1'], ['chat1', 'chat2']. When provided, chat search will only match results on these chats. Empty to search all accessible chats.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub chat_ids: Vec<String>,

    /// A list of project ids to search within. Examples: ['project1']. Empty to ignore project filtering.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub project_ids: Vec<String>,

    /// Filter by chat owner principal — a user ('macro|user1@user.com'), a bot
    /// ('bot|<uuid>'), or a team (a bare hyphenated uuid). Examples:
    /// ['macro|user1@user.com'], ['macro|user1@user.com', 'bot|0199...']. Empty to
    /// search all owners.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub owners: Vec<String>,

    /// Filter by chat importance. None to ignore, true to pass through (no clause), false to short-circuit and return nothing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub importance: Option<bool>,

    /// Filter by chat notification state.
    #[serde(default, skip_serializing_if = "NotificationFilters::is_empty")]
    pub notification_filters: NotificationFilters,
}

impl IsEmpty for ChatFilters {
    fn is_empty(&self) -> bool {
        let ChatFilters {
            role,
            chat_ids,
            project_ids,
            owners,
            importance,
            notification_filters,
        } = self;
        role.is_empty()
            && chat_ids.is_empty()
            && project_ids.is_empty()
            && owners.is_empty()
            && importance.is_none()
            && notification_filters.is_empty()
    }
}

/// Viewer-relative attendance status for a call record.
/// Serializes as `ATTENDED`, `MISSED`, or `UNATTENDED`.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Copy)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[cfg_attr(feature = "schema", derive(utoipa::ToSchema, schemars::JsonSchema))]
pub enum CallStatus {
    /// The viewer is a call participant.
    Attended,
    /// The viewer is not a call participant and is in the call's channel.
    Missed,
    /// The viewer is not a call participant and is not in the call's channel.
    Unattended,
}

/// Filters for call records.
#[derive(Debug, Serialize, Deserialize, Default, PartialEq, Clone)]
#[cfg_attr(feature = "schema", derive(utoipa::ToSchema, schemars::JsonSchema))]
pub struct CallFilters {
    /// Call record IDs to filter by. Empty to include all calls.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub call_ids: Vec<String>,
    /// Channel IDs to filter calls by. Empty to include all calls.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub channel_ids: Vec<String>,
    /// Speaker macro user ids. Empty to include all.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub speaker_ids: Vec<String>,
    /// Filter by the requesting user's viewer-relative call status.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<CallStatus>,
    /// Legacy filter by whether the requesting user attended the call.
    /// Prefer [`CallFilters::status`] for new callers.
    /// `None` = no filter, `Some(true)` = only calls the user joined,
    /// `Some(false)` = only calls the user did not join.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attended: Option<bool>,
}

impl IsEmpty for CallFilters {
    fn is_empty(&self) -> bool {
        let CallFilters {
            call_ids,
            channel_ids,
            speaker_ids,
            status,
            attended,
        } = self;
        call_ids.is_empty()
            && channel_ids.is_empty()
            && speaker_ids.is_empty()
            && status.is_none()
            && attended.is_none()
    }
}

/// Filters for initiatives.
#[derive(Debug, Serialize, Deserialize, Default, PartialEq, Clone)]
#[cfg_attr(feature = "schema", derive(utoipa::ToSchema, schemars::JsonSchema))]
pub struct InitiativeFilters {
    /// Opt this query into initiatives at all. Initiatives are off by
    /// default — see [`crate::ast::initiative::InitiativeLiteral::Include`].
    /// Asking for specific `initiative_ids` or `owners` also opts in.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub include: bool,
    /// Initiative ids to filter by. Empty to include all accessible initiatives.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub initiative_ids: Vec<String>,
    /// Filter by initiative owner principal — a user ('macro|user1@user.com'), a bot
    /// ('bot|<uuid>'), or a team (a bare hyphenated uuid). Empty to include every
    /// owner.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub owners: Vec<String>,
    /// Case-insensitive name substring.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Inclusive upper due-date bound.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub due_before: Option<chrono::DateTime<chrono::Utc>>,
    /// Inclusive lower due-date bound.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub due_after: Option<chrono::DateTime<chrono::Utc>>,
}

impl IsEmpty for InitiativeFilters {
    fn is_empty(&self) -> bool {
        let InitiativeFilters {
            include,
            initiative_ids,
            owners,
            name,
            due_before,
            due_after,
        } = self;
        !include
            && initiative_ids.is_empty()
            && owners.is_empty()
            && name.is_none()
            && due_before.is_none()
            && due_after.is_none()
    }
}

/// Filters for agent sessions.
#[derive(Debug, Serialize, Deserialize, Default, PartialEq, Clone)]
#[cfg_attr(feature = "schema", derive(utoipa::ToSchema, schemars::JsonSchema))]
pub struct AgentSessionFilters {
    /// Opt this query into agent sessions at all. Agent sessions are off by
    /// default — see [`crate::ast::agent_session::AgentSessionLiteral::Include`].
    /// Asking for specific `ids` or `owners` also opts in.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub include: bool,
    /// Agent session ids to filter by. Empty to include all accessible sessions.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ids: Vec<String>,
    /// Filter by session owner principal — a user ('macro|user1@user.com'), a bot
    /// ('bot|<uuid>'), or a team (a bare hyphenated uuid). Empty to include every
    /// owner.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub owners: Vec<String>,
}

impl IsEmpty for AgentSessionFilters {
    fn is_empty(&self) -> bool {
        let AgentSessionFilters {
            include,
            ids,
            owners,
        } = self;
        !include && ids.is_empty() && owners.is_empty()
    }
}

/// Filters for foreign entity records.
#[derive(Debug, Serialize, Deserialize, Default, PartialEq, Clone)]
#[cfg_attr(feature = "schema", derive(utoipa::ToSchema, schemars::JsonSchema))]
pub struct ForeignEntityFilters {
    /// Internal foreign entity record IDs to filter by. Empty to include all records.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ids: Vec<String>,
    /// External entity identifiers to filter by. Empty to include all external IDs.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub foreign_entity_ids: Vec<String>,
    /// External source names to filter by. Empty to include all sources.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub foreign_entity_sources: Vec<String>,
    /// When true, only return foreign entities whose metadata lists the requesting user as a
    /// participant (GitHub `involves:me` semantics for `github_pull_request` records). False or
    /// absent applies no filter. Serialized in filter ASTs as the `"me"` literal.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub includes_me: bool,

    /// Filter by the notification state of the foreign entity (e.g. whether the requesting user's
    /// GitHub PR notification is done/seen).
    #[serde(default, skip_serializing_if = "NotificationFilters::is_empty")]
    pub notification_filters: NotificationFilters,
}

impl IsEmpty for ForeignEntityFilters {
    fn is_empty(&self) -> bool {
        let ForeignEntityFilters {
            ids,
            foreign_entity_ids,
            foreign_entity_sources,
            includes_me,
            notification_filters,
        } = self;
        ids.is_empty()
            && foreign_entity_ids.is_empty()
            && foreign_entity_sources.is_empty()
            && !*includes_me
            && notification_filters.is_empty()
    }
}

/// The channel message filters used to filter down what channel messages you search over.
#[derive(Debug, Serialize, Deserialize, Default, PartialEq, Clone)]
#[cfg_attr(feature = "schema", derive(utoipa::ToSchema, schemars::JsonSchema))]
pub struct ChannelFilters {
    /// Channel thread IDs to search within. Examples: ['thread123']. Empty to search all threads.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub thread_ids: Vec<String>,
    /// Channel user mentions to search for. Examples: ['@username']. Empty if not filtering by mentions.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mentions: Vec<String>,
    /// Channel organization ID to search within. Empty to ignore organization filtering.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub org_id: Option<i64>,
    /// Channel team ID to search within. Empty to ignore team filtering.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub team_id: Option<String>,
    /// Channel IDs to search within. Examples: ['general']. Empty to search all accessible channels.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub channel_ids: Vec<String>,
    /// Sender IDs to search within. Examples: ['user1']. Empty to search all accessible senders.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sender_ids: Vec<String>,

    /// Channel types to filter by. Examples: ['public'], ['direct_message', 'private']. Empty to search all channel types.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub channel_types: Vec<String>,

    /// Filter by channel importance. None to ignore, true to pass through (no clause), false to short-circuit and return nothing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub importance: Option<bool>,

    /// Filter by whether the requesting user is an active participant of the channel.
    /// Presence of this filter also widens the candidate set to team channels of the
    /// user's teams that they have not joined, so `false` matches those channels.
    /// None to ignore (participant channels only, today's default).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_participant: Option<bool>,

    /// Filter by channel notification state.
    #[serde(default, skip_serializing_if = "NotificationFilters::is_empty")]
    pub notification_filters: NotificationFilters,
}

impl IsEmpty for ChannelFilters {
    fn is_empty(&self) -> bool {
        let ChannelFilters {
            thread_ids,
            mentions,
            org_id,
            team_id,
            channel_ids,
            sender_ids,
            channel_types,
            importance,
            is_participant,
            notification_filters,
        } = self;
        thread_ids.is_empty()
            && mentions.is_empty()
            && org_id.is_none()
            && team_id.is_none()
            && channel_ids.is_empty()
            && sender_ids.is_empty()
            && channel_types.is_empty()
            && importance.is_none()
            && is_participant.is_none()
            && notification_filters.is_empty()
    }
}

/// The channel thread filters used to filter down channel-thread entities.
#[derive(Debug, Serialize, Deserialize, Default, PartialEq, Clone)]
#[cfg_attr(feature = "schema", derive(utoipa::ToSchema, schemars::JsonSchema))]
pub struct ChannelThreadFilters {
    /// Channel thread parent message IDs to include. Empty to include all accessible threads.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub thread_ids: Vec<String>,

    /// Channel IDs containing the thread. Empty to include threads from all accessible channels.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub channel_ids: Vec<String>,

    /// Sender IDs for the root thread message. Empty to include all root senders.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub root_sender_ids: Vec<String>,

    /// Thread participant user IDs. A participant is the root message sender, anyone
    /// who replied in the thread, or anyone @-mentioned in the thread (group mentions
    /// like @here count through their per-user expansion at send time). Empty to
    /// include threads regardless of participants.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub participant_ids: Vec<String>,
}

impl IsEmpty for ChannelThreadFilters {
    fn is_empty(&self) -> bool {
        let ChannelThreadFilters {
            thread_ids,
            channel_ids,
            root_sender_ids,
            participant_ids,
        } = self;
        thread_ids.is_empty()
            && channel_ids.is_empty()
            && root_sender_ids.is_empty()
            && participant_ids.is_empty()
    }
}

/// A single property-based filter condition.
///
/// Each filter targets a specific property definition on entities of a given type,
/// matching against select option UUIDs or entity reference IDs.
/// Multiple values within a single filter are OR'd together.
/// Multiple filters are AND'd together.
#[derive(Debug, Serialize, Deserialize, Default, PartialEq, Clone)]
#[cfg_attr(feature = "schema", derive(utoipa::ToSchema, schemars::JsonSchema))]
pub struct PropertyFilter {
    /// The UUID of the property definition to filter on.
    pub property_definition_id: String,
    /// The entity type for the property lookup (e.g., "TASK", "DOCUMENT", "PROJECT").
    /// When None, matches across all entity types.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entity_type: Option<String>,
    /// Select option UUIDs to match. Multiple values are OR'd together.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub option_ids: Vec<String>,
    /// Entity reference IDs to match. Multiple values are OR'd together.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub entity_ids: Vec<String>,
}

impl IsEmpty for PropertyFilter {
    fn is_empty(&self) -> bool {
        self.option_ids.is_empty() && self.entity_ids.is_empty()
    }
}

/// The project filters used to filter down what projects you search over.
#[derive(Debug, Serialize, Deserialize, Default, PartialEq, Clone)]
#[cfg_attr(feature = "schema", derive(utoipa::ToSchema, schemars::JsonSchema))]
pub struct ProjectFilters {
    /// Project IDs to search within. Examples: ['project1']. Empty to search all accessible projects.
    /// By default matches children of these projects; set `include_root` to also match the projects themselves.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub project_ids: Vec<String>,

    /// When true, `project_ids` also matches the projects themselves in addition to their children.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub include_root: bool,

    /// Filter by project owner principal — a user ('macro|user1@user.com'), a bot
    /// ('bot|<uuid>'), or a team (a bare hyphenated uuid). Examples:
    /// ['macro|user1@user.com'], ['macro|user1@user.com', 'bot|0199...']. Empty to
    /// search all owners.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub owners: Vec<String>,

    /// Filter by project importance. None to ignore, true to pass through (no clause), false to short-circuit and return nothing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub importance: Option<bool>,

    /// Filter by project notification state.
    #[serde(default, skip_serializing_if = "NotificationFilters::is_empty")]
    pub notification_filters: NotificationFilters,
}

impl IsEmpty for ProjectFilters {
    fn is_empty(&self) -> bool {
        let ProjectFilters {
            project_ids,
            include_root: _,
            owners,
            importance,
            notification_filters,
        } = self;
        project_ids.is_empty()
            && owners.is_empty()
            && importance.is_none()
            && notification_filters.is_empty()
    }
}

/// How multiple `tag_option_ids` combine when filtering.
#[derive(Debug, Copy, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(utoipa::ToSchema, schemars::JsonSchema))]
pub enum TagFilterMode {
    /// Match entities holding at least one of the selected tags (default).
    #[default]
    Any,
    /// Match entities holding every selected tag.
    All,
}

/// a bundle of all of the filters for each entity type
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(utoipa::ToSchema, schemars::JsonSchema))]
pub struct EntityFilters {
    /// Restrict results to the authenticated viewer's favorites when true.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub favorites_only: Option<bool>,
    /// the bundled [ProjectFilters]
    #[serde(default)]
    pub project_filters: ProjectFilters,
    /// the bundled [DocumentFilters]
    #[serde(default)]
    pub document_filters: DocumentFilters,
    /// the bundled [ChatFilters]
    #[serde(default)]
    pub chat_filters: ChatFilters,
    /// the bundled [ChannelFilters]
    #[serde(default)]
    pub channel_filters: ChannelFilters,
    /// the bundled [ChannelThreadFilters]
    #[serde(default)]
    pub channel_thread_filters: ChannelThreadFilters,
    /// the bundled [CallFilters]
    #[serde(default)]
    pub call_filters: CallFilters,
    /// the bundled [ForeignEntityFilters]
    #[serde(default)]
    pub foreign_entity_filters: ForeignEntityFilters,
    /// Initiative filters. Initiatives are opt-in.
    #[serde(default)]
    pub initiative_filters: InitiativeFilters,
    /// the bundled [AgentSessionFilters]
    #[serde(default)]
    pub agent_session_filters: AgentSessionFilters,
    /// property-based filters applied across entity types
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub property_filters: Vec<PropertyFilter>,
    /// tag option ids matched against `properties.values`, OR'd together across
    /// all tag definitions. Option ids are globally unique, so the match ignores
    /// the owning definition id (personal and team tags combine into one OR).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tag_option_ids: Vec<String>,
    /// How the `tag_option_ids` combine: `any` (default) matches entities
    /// holding at least one selected tag, `all` requires every selected tag.
    #[serde(default)]
    pub tag_filter_mode: TagFilterMode,
}

impl IsEmpty for EntityFilters {
    fn is_empty(&self) -> bool {
        let EntityFilters {
            favorites_only,
            project_filters,
            document_filters,
            chat_filters,
            channel_filters,
            channel_thread_filters,
            call_filters,
            foreign_entity_filters,
            agent_session_filters,
            initiative_filters,
            property_filters,
            tag_option_ids,
            // Mode is a modifier on tag_option_ids, not a filter by itself.
            tag_filter_mode: _,
        } = self;
        favorites_only != &Some(true)
            && project_filters.is_empty()
            && document_filters.is_empty()
            && chat_filters.is_empty()
            && channel_filters.is_empty()
            && channel_thread_filters.is_empty()
            && call_filters.is_empty()
            && foreign_entity_filters.is_empty()
            && agent_session_filters.is_empty()
            && initiative_filters.is_empty()
            && property_filters.iter().all(IsEmpty::is_empty)
            && tag_option_ids.is_empty()
    }
}
