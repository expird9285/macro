//! Configuration for the agent harness service, loaded via the standard
//! `macro_config` pattern so it gets a `doppler_config` validation binary.
//!
//! Required env vars are declared here as typed fields. The `doppler_config`
//! binary loads this `Config` from Doppler for both the dev and prod
//! environments, surfacing any missing or mistyped values at CI time.

use anyhow::Context;
use database_env_vars::{DatabaseUrl, RedisUri};
use entity_registry::NonUserOwners;
pub use macro_env::Environment;
use macro_uuid::Uuid;

use secretsmanager_client::LocalOrRemoteSecret;

macro_env_var::env_vars!(
    /// Local static file bucket, read only when using local AWS.
    pub struct StaticStorageBucket;
    /// Comma-separated Kafka bootstrap servers.
    #[derive(Clone)]
    pub struct KafkaBrokers;
    /// PEM private key of the GitHub App installation tokens are minted with.
    pub struct GithubSyncAppPemSecretKey;
    /// RSA key Macro API tokens are signed with - the same one
    /// `authentication_service` signs with. The egress proxy mints
    /// short-lived tokens for session owners inline.
    pub struct MacroApiTokenPrivateSecretKey;
    /// Issuer stamped into minted Macro API tokens; must match what the
    /// validators expect.
    pub struct MacroApiTokenIssuer;
    /// OAuth client ID for the Pipedream API. The same credentials
    /// `document_cognition_service` uses: the connections a sandbox spends
    /// are the ones the person connected in Macro, in the same rows.
    pub struct PipedreamClientId;
    /// OAuth client secret for the Pipedream API.
    pub struct PipedreamClientSecret;
    /// The Pipedream Connect project ID (`proj_...`).
    pub struct PipedreamProjectId;
    /// AES key the `mcp_servers` rows' OAuth credentials are encrypted with -
    /// the same one `document_cognition_service` writes them with, so the
    /// custom MCP servers a person added in Macro are the ones the egress
    /// proxy can refresh and stamp for their sandboxes.
    pub struct McpCredentialsKeySecretName;
);

macro_env_var::maybe_env_vars!(
    /// PostHog project token. Absent deployments retain standard agent tools.
    pub struct PosthogApiKey;
    /// Regional PostHog ingestion origin; defaults to the US origin.
    pub struct PosthogHost;
    /// Rollout gate for sessions owned by a bot.
    pub struct EnableNonUserOwners;
    /// The authentication service's own internal key, which settlement
    /// requests for counted usage present. Required while
    /// `ENABLE_AI_USAGE_BILLING` is true; otherwise unused.
    pub struct AuthenticationServiceSecretKey;
);

/// The Pipedream project environment matching this deployment: production in
/// prd, development everywhere else.
fn default_pipedream_environment() -> String {
    match Environment::new_or_prod() {
        Environment::Production => "production".to_owned(),
        _ => "development".to_owned(),
    }
}

/// The configuration parameters for the agent harness service.
#[derive(macro_config::MacroConfig)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub struct Config {
    /// Hide agent DM contexts from standalone lists. Enable only after all
    /// replicas use the independent for_warming lifecycle marker.
    #[macro_config_default(false)]
    pub hide_agent_dm_sessions: bool,
    /// Backend feature flags, evaluated using the authenticated session owner.
    pub posthog_api_key: PosthogApiKey,
    /// Same regional origin as the frontend's PostHog project.
    pub posthog_host: PosthogHost,
    /// Pinned Deno executable. Unset hosts do not advertise code execution.
    pub deno_binary: Option<String>,
    /// Default-off quota admission and prospective usage counting.
    #[macro_config_default(ai_usage::AiUsageEnforcement::Disabled)]
    pub enable_ai_usage_enforcement: ai_usage::AiUsageEnforcement,
    /// Default-off settlement of usage past allowances. When enabled, counted
    /// usage recorded here asks the authentication service to settle; that
    /// service's own policy decides whether it does.
    #[macro_config_default(ai_billing::AiUsageBilling::Disabled)]
    pub enable_ai_usage_billing: ai_billing::AiUsageBilling,
    /// Key settlement requests present to the authentication service.
    pub authentication_service_secret_key: AuthenticationServiceSecretKey,
    /// The free plan's hard monthly AI cap, in cents at provider cost.
    /// Mandatory; set in Doppler.
    pub ai_usage_free_included_allowance_cents: ai_billing::IncludedAllowanceCents,
    /// In-plan AI allowance per Premium seat per period, in cents at provider
    /// cost. Mandatory; set in Doppler.
    pub ai_usage_included_allowance_cents: ai_billing::IncludedAllowanceCents,
    /// In-plan AI allowance per Max seat per period, in cents at provider
    /// cost. Mandatory; set in Doppler.
    pub ai_usage_max_included_allowance_cents: ai_billing::IncludedAllowanceCents,
    /// Markup on paid AI usage past the allowance, as a whole percent of
    /// provider cost. Mandatory; set in Doppler.
    pub ai_usage_overage_markup_percent: ai_billing::OverageMarkupPercent,
    /// The environment we are in.
    #[macro_config_default(Environment::new_or_prod())]
    pub environment: Environment,
    /// Comma-separated Kafka bootstrap servers.
    pub kafka_brokers: KafkaBrokers,
    /// MacroDB connection string; `agent_sessions` lives here.
    pub database_url: DatabaseUrl,
    /// Shared Redis used for cross-replica command forwarding.
    pub redis_uri: RedisUri,
    /// The bot this deployment answers for.
    pub harness_bot_id: Uuid,
    /// Model slug stamped onto sessions this deployment opens.
    #[macro_config_default(String::from("claude"))]
    pub harness_model: String,
    /// Harness slug stamped onto sessions this deployment opens.
    #[macro_config_default(String::from("opencode"))]
    pub harness_slug: String,
    /// Provider-qualified model id stamped onto sessions the in-memory bot opens.
    #[macro_config_default(String::from("anthropic/claude-sonnet-5-5"))]
    pub inmem_model: String,
    /// Harness slug stamped onto sessions the in-memory bot opens.
    #[macro_config_default(String::from("macro-inmem"))]
    pub inmem_harness_slug: String,
    /// Key for internal service-to-service calls (the connection gateway).
    pub internal_api_key: String,
    /// Key required by document storage's internal endpoints.
    pub document_storage_service_auth_key: String,
    /// Port the control routes are served on.
    #[macro_config_default(8101)]
    pub port: u16,
    /// Port the sandbox-facing egress proxy is served on.
    ///
    /// The shared gateway forwards `/agent-harness-egress/*` to this listener.
    /// The egress router reads sandbox session tokens from `Authorization`.
    #[macro_config_default(8102)]
    pub egress_port: u16,
    /// OAuth client ID for the Pipedream API.
    pub pipedream_client_id: PipedreamClientId,
    /// OAuth client secret for the Pipedream API.
    pub pipedream_client_secret: PipedreamClientSecret,
    /// The Pipedream Connect project ID.
    pub pipedream_project_id: PipedreamProjectId,
    /// The Pipedream project environment (`development` or `production`).
    #[macro_config_default(default_pipedream_environment())]
    pub pipedream_environment: String,
    /// Base URL of the Pipedream API.
    #[macro_config_default(String::from(pipedream_mcp::outbound::api::DEFAULT_API_URL))]
    pub pipedream_api_url: String,
    /// URL of Pipedream's remote MCP server.
    #[macro_config_default(String::from(pipedream_mcp::outbound::api::DEFAULT_MCP_URL))]
    pub pipedream_mcp_url: String,
    /// RSA key Macro API tokens are signed with.
    pub macro_api_token_private_secret_key: LocalOrRemoteSecret<MacroApiTokenPrivateSecretKey>,
    /// AES key the custom MCP servers' stored OAuth credentials are encrypted with.
    pub mcp_credentials_key_secret_name: LocalOrRemoteSecret<McpCredentialsKeySecretName>,
    /// Issuer stamped into minted Macro API tokens.
    pub macro_api_token_issuer: MacroApiTokenIssuer,
    /// S3 bucket of pull request patches, owned by document-storage-service and
    /// shared with it for GitHub PR review.
    pub github_pull_request_patch_bucket: String,
    /// Client id of the GitHub App installation tokens are minted for.
    pub github_sync_app_client_id: String,
    /// PEM private key of that App.
    pub github_sync_app_pem_secret_key: LocalOrRemoteSecret<GithubSyncAppPemSecretKey>,
    /// Lets a team-scoped bot with no acting user own the sessions it creates.
    pub enable_non_user_owners: EnableNonUserOwners,
}

impl Config {
    /// The AI pricing every billing component is composed with. Every value
    /// is validated when the configuration loads.
    pub fn ai_pricing(&self) -> ai_billing::AiPricing {
        ai_billing::AiPricing::new(
            ai_billing::PlanAllowances {
                free: self.ai_usage_free_included_allowance_cents,
                premium: self.ai_usage_included_allowance_cents,
                max: self.ai_usage_max_included_allowance_cents,
            },
            self.ai_usage_overage_markup_percent,
        )
    }

    /// The parsed `ENABLE_NON_USER_OWNERS` gate. Missing is disabled.
    pub fn non_user_owners(&self) -> anyhow::Result<NonUserOwners> {
        NonUserOwners::from_config_value(self.enable_non_user_owners.value())
            .context("ENABLE_NON_USER_OWNERS must be `true` or `false`")
    }

    /// Where this host's settlement requests go: the authentication service
    /// while `ENABLE_AI_USAGE_BILLING` is true, which then requires
    /// `AUTHENTICATION_SERVICE_SECRET_KEY`; nowhere otherwise.
    pub fn settlement_route(&self) -> anyhow::Result<ai_billing::composition::SettlementRoute> {
        let client = self
            .authentication_service_secret_key
            .value()
            .filter(|key| !key.trim().is_empty())
            .map(|key| {
                anyhow::Ok(std::sync::Arc::new(
                    authentication_service_client::AuthServiceClient::new(
                        key.to_owned(),
                        macro_service_urls::AuthServiceUrl::new()?.to_string(),
                    ),
                ))
            })
            .transpose()?;
        Ok(ai_billing::composition::SettlementRoute::from_policy(
            self.enable_ai_usage_billing,
            client,
        )?)
    }

    /// Load the configuration from the environment.
    pub fn from_env() -> anyhow::Result<Self> {
        let enforcement = ai_usage::config::load_ai_usage_enforcement()
            .map_err(|error| anyhow::anyhow!("{error}"))?;
        let billing = ai_billing::config::load_ai_usage_billing()
            .map_err(|error| anyhow::anyhow!("{error}"))?;
        let mut config = macro_config::ConfigLoader::load::<Config>()
            .context("failed to load agent harness service config")?;
        config.enable_ai_usage_enforcement = enforcement;
        config.enable_ai_usage_billing = billing;
        Ok(config)
    }
}
