use std::sync::LazyLock;

use crate::account_link_state::AccountLinkStateKey;
use crate::service::signup_policy::SignupPolicy;
use anyhow::Context;
use database_env_vars::{DatabaseUrl, RedisUri};
use gtm_invite::domain::models::{GtmInviteConfig, PromoCode};
use macro_auth::InternalApiKey;
pub use macro_env::Environment;
use macro_env_var::{env_vars, maybe_env_vars};

// BASE_URL config value. This is validated when creating the config in main.rs
pub static BASE_URL: LazyLock<String> = LazyLock::new(|| {
    BaseUrl::new()
        .expect("BASE_URL must be provided via APP_SECRETS_JSON or env")
        .as_ref()
        .to_string()
});

env_vars! {
    pub struct BaseUrl;
    pub struct FusionAuthApiSecretKey;
    pub struct FusionAuthClientId;
    pub struct FusionAuthClientSecretKey;
    pub struct FusionAuthBaseUrl;
    pub struct FusionAuthOauthRedirectUri;
    pub struct GoogleClientId;
    pub struct GoogleClientSecretKey;
    pub struct StripeSecretKey;
    pub struct ServiceInternalAuthKey;
    pub struct GithubClientId;
    pub struct GithubClientSecret;
    pub struct GithubIdpId;
    /// HMAC-SHA256 key for the signed `state` on account-link OAuth requests:
    /// at least 32 bytes of random data. See `account_link_state`.
    pub struct AccountLinkStateSecret;
    pub struct StripePriceId;
    /// Comma-separated Kafka bootstrap servers for the macro event broker.
    pub struct KafkaBrokers;
}

maybe_env_vars! {
    /// Browser-reachable FusionAuth origin used for OAuth authorization redirects.
    pub struct FusionAuthPublicUrl;
    pub struct GaMeasurementId;
    pub struct GaApiSecret;
    pub struct MetaPixelId;
    pub struct MetaAccessToken;
    pub struct MetaTestEventCode;
    pub struct PosthogApiKey;
    pub struct PosthogHost;
    pub struct LoopsApiKey;
    /// JSON array of exact email addresses allowed to sign up in Develop.
    pub struct DevelopmentSignupAllowlistJson;
    /// Stripe promotion code GTM invite links grant at checkout. Defaults to `1MF`.
    pub struct GtmInvitePromoCode;
    /// Hours a GTM invite link stays openable after creation. Defaults to 48.
    pub struct GtmInviteLinkTtlHours;
    /// Stripe price id for the Max plan seat. Optional so the service can
    /// deploy before the price exists in Stripe; until it is set, Max checkout
    /// and plan changes answer 400 and every subscription maps to Premium.
    pub struct StripeMaxPriceId;
}

/// The configuration parameters for the application.
///
/// These can either be passed on the command line, or pulled from environment variables.
/// The latter is preferred as environment variables are one of the recommended ways to
/// populate the Docker container
///
/// See `.env.sample` in document-storage-service root for details.
#[derive(macro_config::MacroConfig)]
// #[macro_config::from_ref_all]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub struct Config {
    /// Default-off quota admission and prospective usage counting.
    #[macro_config_default(ai_billing::AiUsageEnforcement::Disabled)]
    pub enable_ai_usage_enforcement: ai_billing::AiUsageEnforcement,
    /// Default-off settlement of usage past allowances: prepaid credit
    /// consumption and Stripe overage collection. This service owns Stripe, so
    /// its policy decides every settlement, however it was requested.
    #[macro_config_default(ai_billing::AiUsageBilling::Disabled)]
    pub enable_ai_usage_billing: ai_billing::AiUsageBilling,
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
    #[allow(dead_code)]
    pub base_url: BaseUrl,
    /// The connection URL for the Postgres database this application should use.
    pub database_url: DatabaseUrl,
    /// The Redis URI for the Redis this application should use.
    pub redis_uri: RedisUri,
    /// FusionAuth API key secret name
    pub fusionauth_api_key_secret_key: FusionAuthApiSecretKey,
    /// FusionAuth client id
    pub fusionauth_client_id: FusionAuthClientId,
    /// FusionAuth client secret key
    pub fusionauth_client_secret_key: FusionAuthClientSecretKey,
    /// FusionAuth base url
    pub fusionauth_base_url: FusionAuthBaseUrl,
    /// Browser-reachable FusionAuth URL. Falls back to the API base URL when unset.
    pub fusionauth_public_url: FusionAuthPublicUrl,
    /// FusionAuth oauth redirect uri
    pub fusionauth_oauth_redirect_uri: FusionAuthOauthRedirectUri,
    /// Google client id
    pub google_client_id: GoogleClientId,
    /// Google client secret key
    pub google_client_secret_key: GoogleClientSecretKey,
    /// Stripe secret key
    pub stripe_secret_key: StripeSecretKey,
    /// The port to listen for HTTP requests on.
    #[macro_config_default(8080)]
    pub port: usize,
    /// The environment we are in
    #[macro_config_default(Environment::new_or_prod())]
    pub environment: Environment,
    /// The internal auth key used by other services
    pub service_internal_auth_key: ServiceInternalAuthKey,
    /// The github client id
    pub github_client_id: GithubClientId,
    /// The github client secret
    pub github_client_secret: GithubClientSecret,
    /// The github idp id
    pub github_idp_id: GithubIdpId,
    /// Signs the `state` carried through account-link OAuth flows. Checked
    /// for length by [`Config::account_link_state_key`] at startup.
    pub account_link_state_secret: AccountLinkStateSecret,
    /// GA4 Measurement ID (optional, e.g., "G-XXXXXXXXXX")
    pub ga_measurement_id: GaMeasurementId,
    /// GA4 Measurement Protocol API secret (optional)
    pub ga_api_secret: GaApiSecret,
    /// Meta Pixel ID (optional)
    pub meta_pixel_id: MetaPixelId,
    /// Meta Conversions API access token (optional)
    pub meta_access_token: MetaAccessToken,
    /// Meta test event code for testing (optional)
    pub meta_test_event_code: MetaTestEventCode,
    /// PostHog API key (optional)
    pub posthog_api_key: PosthogApiKey,
    /// PostHog host (optional)
    pub posthog_host: PosthogHost,
    /// Loops API key (optional). When set, Macro sign-ups are added to our
    /// Loops audience.
    pub loops_api_key: LoopsApiKey,
    /// JSON array of exact non-Macro email addresses allowed to sign up in Develop.
    ///
    /// All `@macro.com` email addresses are allowed by the Develop policy automatically.
    pub development_signup_allowlist_json: DevelopmentSignupAllowlistJson,
    /// Whether Develop allows every public signup without reading
    /// `DEVELOPMENT_SIGNUP_ALLOWLIST_JSON`. Production and Local ignore it.
    #[macro_config_default(false)]
    pub development_bypass_signup_allowlist: bool,
    /// Stripe promotion code applied at checkout for accounts that signed up
    /// through a GTM invite link (optional, defaults to `1MF`).
    pub gtm_invite_promo_code: GtmInvitePromoCode,
    /// Hours a GTM invite link can be opened and redeemed (optional, defaults to 48).
    pub gtm_invite_link_ttl_hours: GtmInviteLinkTtlHours,
    /// The stripe price id for the Premium plan seat
    pub stripe_price_id: StripePriceId,
    /// The stripe price id for the Max plan seat (optional, see
    /// [`StripeMaxPriceId`])
    pub stripe_max_price_id: StripeMaxPriceId,
    /// The internal api key
    pub internal_api_key: InternalApiKey,
    /// Comma-separated Kafka bootstrap servers for the macro event broker.
    pub kafka_brokers: KafkaBrokers,
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

    pub fn from_env() -> anyhow::Result<Self> {
        let enforcement = ai_usage::config::load_ai_usage_enforcement()
            .map_err(|error| anyhow::anyhow!("{error}"))?;
        let billing = ai_billing::config::load_ai_usage_billing()
            .map_err(|error| anyhow::anyhow!("{error}"))?;
        let mut config = macro_config::ConfigLoader::load::<Config>()
            .context("failed to load authentication service config")?;
        config.enable_ai_usage_enforcement = enforcement;
        config.enable_ai_usage_billing = billing;
        Ok(config)
    }

    /// The key that signs and verifies account-link OAuth state.
    ///
    /// # Errors
    /// If the configured secret is blank or shorter than 32 bytes.
    pub(crate) fn account_link_state_key(&self) -> anyhow::Result<AccountLinkStateKey> {
        AccountLinkStateKey::new(self.account_link_state_secret.as_ref())
    }

    /// Checks the account-link signing secret without exposing it. Public for
    /// the Doppler config check binary.
    pub fn validate_account_link_state_secret(&self) -> anyhow::Result<()> {
        self.account_link_state_key().map(drop)
    }

    /// Resolves the signup policy for the configured environment.
    pub(crate) fn signup_policy(&self) -> anyhow::Result<SignupPolicy> {
        self.signup_policy_for_environment(self.environment)
    }

    /// Resolves the signup policy for an explicit environment. Public for the
    /// Doppler config check binary, which validates both environments.
    pub fn signup_policy_for_environment(
        &self,
        environment: Environment,
    ) -> anyhow::Result<SignupPolicy> {
        // Temporarily open Develop signups even when Doppler disables the bypass.
        // Remove this override to restore the configured allowlist policy.
        let bypass_allowlist =
            self.development_bypass_signup_allowlist || matches!(environment, Environment::Develop);
        resolve_signup_policy(
            environment,
            bypass_allowlist,
            &self.development_signup_allowlist_json,
        )
    }

    /// Resolves the offer GTM invite links carry.
    pub(crate) fn gtm_invite_config(&self) -> anyhow::Result<GtmInviteConfig> {
        resolve_gtm_invite_config(&self.gtm_invite_promo_code, &self.gtm_invite_link_ttl_hours)
    }
}

fn resolve_signup_policy(
    environment: Environment,
    development_bypass_signup_allowlist: bool,
    development_signup_allowlist_json: &DevelopmentSignupAllowlistJson,
) -> anyhow::Result<SignupPolicy> {
    match environment {
        Environment::Production | Environment::Local => Ok(SignupPolicy::allow_all()),
        Environment::Develop if development_bypass_signup_allowlist => {
            Ok(SignupPolicy::allow_all())
        }
        Environment::Develop => {
            let raw_allowlist = nonblank_value(development_signup_allowlist_json.value())
                .context("DEVELOPMENT_SIGNUP_ALLOWLIST_JSON is required in Develop")?;
            SignupPolicy::from_allowlist_json(raw_allowlist)
                .context("DEVELOPMENT_SIGNUP_ALLOWLIST_JSON is invalid")
        }
    }
}

/// The promotion code applied when `GTM_INVITE_PROMO_CODE` is unset: 100% off
/// the first month, created in Stripe for exactly this program.
const DEFAULT_GTM_INVITE_PROMO_CODE: &str = "1MF";
/// How long an invite link stays usable when `GTM_INVITE_LINK_TTL_HOURS` is unset.
const DEFAULT_GTM_INVITE_LINK_TTL_HOURS: i64 = 48;
/// Free months the default promotion grants, for user-facing copy.
const GTM_INVITE_FREE_MONTHS: u8 = 1;

fn resolve_gtm_invite_config(
    promo_code: &GtmInvitePromoCode,
    link_ttl_hours: &GtmInviteLinkTtlHours,
) -> anyhow::Result<GtmInviteConfig> {
    let promo_code: PromoCode = nonblank_value(promo_code.value())
        .unwrap_or(DEFAULT_GTM_INVITE_PROMO_CODE)
        .parse()
        .context("GTM_INVITE_PROMO_CODE is invalid")?;
    let link_ttl_hours: i64 = match nonblank_value(link_ttl_hours.value()) {
        Some(hours) => hours
            .trim()
            .parse()
            .context("GTM_INVITE_LINK_TTL_HOURS must be a whole number of hours")?,
        None => DEFAULT_GTM_INVITE_LINK_TTL_HOURS,
    };
    if link_ttl_hours <= 0 {
        anyhow::bail!("GTM_INVITE_LINK_TTL_HOURS must be positive");
    }
    Ok(GtmInviteConfig {
        promo_code,
        link_ttl: chrono::Duration::hours(link_ttl_hours),
        free_months: GTM_INVITE_FREE_MONTHS,
    })
}

fn nonblank_value(value: Option<&str>) -> Option<&str> {
    value.filter(|value| !value.trim().is_empty())
}

#[cfg(test)]
mod test;
