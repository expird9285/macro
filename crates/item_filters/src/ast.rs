//! This module defines stricter typing for the filters found in lib.
//! This is used to construct a strictly typed ast for the input filters, allowing consumers to have a logical represenation of the required operations

use crate::{
    AgentSessionFilters, CallFilters, ChannelFilters, ChannelThreadFilters, ChatFilters,
    DocumentFilters, EntityFilters, ForeignEntityFilters, InitiativeFilters, ProjectFilters,
    PropertyFilter,
    ast::{
        agent_session::AgentSessionLiteral,
        call::CallLiteral,
        channel::{ChannelLiteral, ChannelThreadLiteral, ChannelTypeFilter},
        chat::{ChatLiteral, ChatRole},
        database_row::DatabaseRowLiteral,
        foreign_entity::ForeignEntityLiteral,
        github_pull_request::GithubPullRequestLiteral,
        initiative::InitiativeLiteral,
        project::ProjectLiteral,
        properties::PropertiesLiteral,
    },
};
use document::DocumentLiteral;
use filter_ast::{ExpandFrame, Expr};
use non_empty::IsEmpty;
use serde::{Deserialize, Serialize};
use std::{marker::PhantomData, sync::Arc};
use thiserror::Error;

/// contains the ast literal value for agent sessions
pub mod agent_session;
/// contains the ast literal value for calls
pub mod call;
/// contains the ast literal value for channels
pub mod channel;
/// contains the ast literal value for chat
pub mod chat;
/// Database row filter literals.
pub mod database_row;
/// contains the date comparison literal type
pub mod date;
/// contains the ast literal value for documents
pub mod document;
/// contains the ast literal value for foreign entities
pub mod foreign_entity;
/// contains the ast literal value for GitHub pull requests
pub mod github_pull_request;
/// Initiative filter literals.
pub mod initiative;
/// contains the ast literal value for projects
pub mod project;
/// contains the ast literal value for property-based filtering
pub mod properties;

#[cfg(test)]
mod tests;

/// encountered an unknown file type
#[derive(Debug, Error)]
#[error("Found unknown value {0} when attempting to parse {t}", t = std::any::type_name::<T>())]
pub struct UnknownValue<T>(String, PhantomData<T>);

trait ParseFromStr: Sized {
    fn parse_from_str<T: AsRef<str>>(s: T) -> Result<Self, UnknownValue<Self>>;
}

/// the types of errors that can occur when expanding [DocumentFilters] into an ast
#[derive(Debug, Error)]
pub enum ExpandErr {
    /// unknown file type
    #[error(transparent)]
    FileTypeErr(#[from] model_file_type::ValueError<model_file_type::FileType>),
    /// unknown chat type
    #[error(transparent)]
    ChatRoleErr(#[from] UnknownValue<ChatRole>),
    /// unknown channel type
    #[error(transparent)]
    ChannelTypeErr(#[from] UnknownValue<ChannelTypeFilter>),
    /// invalid uuid
    #[error("Invalid uuid string: {0}")]
    Uuid(#[from] uuid::Error),
    /// invalid macro user id
    #[error(transparent)]
    MacroIdErr(#[from] macro_user_id::error::ParseErr),
    /// invalid owner principal
    #[error(transparent)]
    OwnerErr(#[from] model_owner::OwnerParseError),
    /// unknown document sub type
    #[error(transparent)]
    DocumentSubTypeErr(#[from] strum::ParseError),
    /// invalid property entity type
    #[error(transparent)]
    PropertyEntityType(#[from] properties::PropertyEntityTypeError),
    /// invalid entity reference id
    #[error(transparent)]
    EntityRefId(#[from] properties::EntityRefIdError),
    /// invalid API AST expansion
    #[error("invalid API AST expansion: {0}")]
    ApiAst(String),
}

/// type alias for a maybe empty, cheaply cloneable ast literal tree
pub type LiteralTree<T> = Option<Arc<Expr<T>>>;

/// Describes a bundle of filters that should be applied across different entity types
#[derive(Debug, Default, Serialize, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(utoipa::ToSchema))]
pub struct EntityFilterAst {
    /// Restrict to the authenticated viewer's favorites before pagination when true.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub favorites_only: Option<bool>,
    /// the filters that should be applied to the document entity
    #[serde(default, rename = "df")]
    #[cfg_attr(feature = "schema", schema(value_type = serde_json::Value))]
    pub document_filter: LiteralTree<DocumentLiteral>,
    /// the filters that should be applied to the project entity
    #[serde(default, rename = "pf")]
    #[cfg_attr(feature = "schema", schema(value_type = serde_json::Value))]
    pub project_filter: LiteralTree<ProjectLiteral>,
    /// the filters that should be applied to the chat entity
    #[serde(default, rename = "cf")]
    #[cfg_attr(feature = "schema", schema(value_type = serde_json::Value))]
    pub chat_filter: LiteralTree<ChatLiteral>,
    /// the filters that should be applied to the channel entity
    #[serde(default, rename = "chanf")]
    #[cfg_attr(feature = "schema", schema(value_type = serde_json::Value))]
    pub channel_filter: LiteralTree<ChannelLiteral>,
    /// the filters that should be applied to the channel-thread entity
    #[serde(default, rename = "cthf")]
    #[cfg_attr(feature = "schema", schema(value_type = serde_json::Value))]
    pub channel_thread_filter: LiteralTree<ChannelThreadLiteral>,
    /// the filters that should be applied to the call entity
    #[serde(default, rename = "callf")]
    #[cfg_attr(feature = "schema", schema(value_type = serde_json::Value))]
    pub call_filter: LiteralTree<CallLiteral>,
    /// the filters that should be applied to foreign entity records
    #[serde(default, rename = "fef")]
    #[cfg_attr(feature = "schema", schema(value_type = serde_json::Value))]
    pub foreign_entity_filter: LiteralTree<ForeignEntityLiteral>,
    /// the filters that should be applied to GitHub pull request records, on top of the foreign
    /// entity filter
    #[serde(default, rename = "ghprf")]
    #[cfg_attr(feature = "schema", schema(value_type = serde_json::Value))]
    pub github_pull_request_filter: LiteralTree<GithubPullRequestLiteral>,
    /// the filters that should be applied to agent sessions
    #[serde(default, rename = "asf")]
    #[cfg_attr(feature = "schema", schema(value_type = serde_json::Value))]
    pub agent_session_filter: LiteralTree<AgentSessionLiteral>,
    /// Initiative filter, absent for queries that do not request initiatives.
    #[serde(default, rename = "if")]
    #[cfg_attr(feature = "schema", schema(value_type = serde_json::Value))]
    pub initiative_filter: LiteralTree<InitiativeLiteral>,
    /// Database row filter, absent for queries that do not request rows.
    #[serde(default, rename = "drf")]
    #[cfg_attr(feature = "schema", schema(value_type = serde_json::Value))]
    pub database_row_filter: LiteralTree<DatabaseRowLiteral>,
    /// the filters that should be applied based on entity properties
    #[serde(default, rename = "propf")]
    #[cfg_attr(feature = "schema", schema(value_type = serde_json::Value))]
    pub properties_filter: LiteralTree<PropertiesLiteral>,
}

impl EntityFilterAst {
    /// expand the input [EntityFilters] into an ast representation
    pub fn new_from_filters(entity_filter: EntityFilters) -> Result<Option<Self>, ExpandErr> {
        if entity_filter.is_empty() {
            return Ok(None);
        }
        Ok(Some(EntityFilterAst {
            favorites_only: entity_filter.favorites_only,
            document_filter: DocumentFilters::expand_ast(entity_filter.document_filters)?
                .map(Arc::new),
            project_filter: ProjectFilters::expand_ast(entity_filter.project_filters)?
                .map(Arc::new),
            chat_filter: ChatFilters::expand_ast(entity_filter.chat_filters)?.map(Arc::new),
            channel_filter: ChannelFilters::expand_ast(entity_filter.channel_filters)?
                .map(Arc::new),
            channel_thread_filter: ChannelThreadFilters::expand_ast(
                entity_filter.channel_thread_filters,
            )?
            .map(Arc::new),
            call_filter: CallFilters::expand_ast(entity_filter.call_filters)?.map(Arc::new),
            foreign_entity_filter: ForeignEntityFilters::expand_ast(
                entity_filter.foreign_entity_filters,
            )?
            .map(Arc::new),
            github_pull_request_filter: None,
            agent_session_filter: AgentSessionFilters::expand_ast(
                entity_filter.agent_session_filters,
            )?
            .map(Arc::new),
            initiative_filter: InitiativeFilters::expand_ast(entity_filter.initiative_filters)?
                .map(Arc::new),
            // Rows are requested through GraphQL only; the REST filters
            // cannot name a table.
            database_row_filter: None,
            properties_filter: Vec::<PropertyFilter>::expand_ast(entity_filter.property_filters)?
                .map(Arc::new),
        }))
    }

    /// mock function to create the an empty ast
    #[cfg(feature = "mock")]
    pub fn mock_empty() -> Self {
        Self {
            favorites_only: None,
            document_filter: None,
            project_filter: None,
            chat_filter: None,
            channel_filter: None,
            channel_thread_filter: None,
            call_filter: None,
            foreign_entity_filter: None,
            github_pull_request_filter: None,
            agent_session_filter: None,
            initiative_filter: None,
            database_row_filter: None,
            properties_filter: None,
        }
    }
}

impl IsEmpty for EntityFilterAst {
    fn is_empty(&self) -> bool {
        let EntityFilterAst {
            favorites_only,
            document_filter,
            project_filter,
            chat_filter,
            channel_filter,
            channel_thread_filter,
            call_filter,
            foreign_entity_filter,
            github_pull_request_filter,
            agent_session_filter,
            initiative_filter,
            database_row_filter,
            properties_filter,
        } = self;
        favorites_only != &Some(true)
            && document_filter.is_none()
            && project_filter.is_none()
            && chat_filter.is_none()
            && channel_filter.is_none()
            && channel_thread_filter.is_none()
            && call_filter.is_none()
            && foreign_entity_filter.is_none()
            && github_pull_request_filter.is_none()
            && agent_session_filter.is_none()
            && initiative_filter.is_none()
            && database_row_filter.is_none()
            && properties_filter.is_none()
    }
}
