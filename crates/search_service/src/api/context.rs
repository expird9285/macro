use agent_session::domain::search::AgentSessionSearchMetadataService;
use axum::extract::FromRef;
use macro_authorization::{
    MacroAuthJwtValidator, MacroAuthorizationServiceImpl, MacroAuthorizationState,
};
use opensearch_client::OpensearchClient;
use readonly_pool::ReadOnlyPool;
use std::sync::Arc;

/// Concrete authorization service backing search request authentication.
pub type SearchAuthorizationService = MacroAuthorizationServiceImpl<MacroAuthJwtValidator>;

/// Minimal state required by search handlers.
/// This can be extracted from any state that implements `FromRef<SearchHandlerState>`.
#[derive(Clone, FromRef)]
pub struct SearchHandlerState {
    /// Read-only database connection for search queries
    pub db: ReadOnlyPool,
    /// OpenSearch client for full-text search
    pub opensearch_client: Arc<OpensearchClient>,
    /// Authorization state used to authenticate search requests.
    pub authorization_state: MacroAuthorizationState<SearchAuthorizationService>,
    /// Agent-session domain service used after the search layer resolves its
    /// authorized session allowlist.
    pub agent_session_search_metadata: Arc<dyn AgentSessionSearchMetadataService>,
}

impl SearchHandlerState {
    /// Assemble the agent-session search use case from read-only adapters.
    pub(crate) fn agent_session_search(
        &self,
    ) -> crate::domain::agent_session::AgentSessionSearchService<
        crate::outbound::agent_session::AgentSessionSearchMetadataSource,
    > {
        crate::domain::agent_session::AgentSessionSearchService(
            crate::outbound::agent_session::AgentSessionSearchMetadataSource {
                db: self.db.clone(),
                service: self.agent_session_search_metadata.clone(),
            },
        )
    }
}
