//! Composition of internal agent session creation over the existing
//! bot directory and routine-session opener.

use std::sync::Arc;

use agent_session::domain::coding_agents::{
    CodingAgentCandidate, CodingAgentDirectory, CodingAgentError, CodingAgentsServiceImpl,
};
use agent_session::domain::routines::RoutineSessions;
use agent_session::inbound::coding_agents::{CodingAgentsState, coding_agents_router};
use axum::Router;
use bots::domain::service::BotService;
use macro_authorization::{MacroAuthorizationService, MacroAuthorizationState};
use macro_user_id::user_id::MacroUserIdStr;

/// Resolves visible agent personas and checks whether their runtime is
/// supported by the in-memory harness.
pub struct AvailableCodingAgents<Bots> {
    bots: Bots,
}

impl<Bots> AvailableCodingAgents<Bots> {
    /// Build the directory over the bots service.
    pub fn new(bots: Bots) -> Self {
        Self { bots }
    }
}

impl<Bots> CodingAgentDirectory for AvailableCodingAgents<Bots>
where
    Bots: BotService,
{
    async fn candidates(
        &self,
        user_id: MacroUserIdStr<'static>,
    ) -> Result<Vec<CodingAgentCandidate>, CodingAgentError> {
        let agents = self
            .bots
            .list_agents(user_id)
            .await
            .map_err(|error| CodingAgentError::Unknown(anyhow::anyhow!(error)))?;

        Ok(agents
            .into_iter()
            .map(|agent| {
                let available = matches!(agent.harness.as_str(), "in-memory" | "macro-inmem");
                CodingAgentCandidate {
                    id: agent.bot.id,
                    name: agent.bot.name,
                    handle: agent.bot.handle,
                    description: agent.description,
                    harness: agent.harness,
                    available,
                    needs_repository: false,
                }
            })
            .collect())
    }
}

/// Build the internal `/internal/coding-agents` router.
pub fn router<Bots, Routines, Auth>(
    directory: AvailableCodingAgents<Bots>,
    routines: Routines,
    authorization: MacroAuthorizationState<Auth>,
) -> Router
where
    Bots: BotService,
    Routines: RoutineSessions,
    Auth: MacroAuthorizationService,
{
    let service = CodingAgentsServiceImpl::new(directory, routines);
    coding_agents_router(CodingAgentsState::new(Arc::new(service), authorization))
}
