//! Postgres-backed permission policies for fixed and database-backed agents.

use agent_harness::domain::model::{AgentKind, PermissionPolicyConfig};
use agent_harness::domain::ports::PermissionPolicySource;
use bot_id::BotId;
use bots::domain::ports::BotRepo;
use bots::outbound::pg_bots_repo::PgBotsRepo;

/// Reads the persona choice for database-backed agents; fixed system bots retain
/// their built-in policy.
#[derive(Clone)]
pub struct PgPermissionPolicySource {
    repo: PgBotsRepo,
}

impl PgPermissionPolicySource {
    /// Build the source over the bots repository.
    pub fn new(repo: PgBotsRepo) -> Self {
        Self { repo }
    }
}

impl PermissionPolicySource for PgPermissionPolicySource {
    async fn permission_policy(&self, bot: BotId) -> anyhow::Result<PermissionPolicyConfig> {
        let agent = self.repo.get_agent(bot).await?;
        let Some(agent) = agent else {
            return Ok(PermissionPolicyConfig::Fixed(AgentKind::of(bot)));
        };
        Ok(PermissionPolicyConfig::Persona {
            kind: AgentKind::for_session(bot, &agent.harness),
            harness_allows_bypass: None,
            auto_accept_permissions: agent.auto_accept_permissions,
        })
    }
}
