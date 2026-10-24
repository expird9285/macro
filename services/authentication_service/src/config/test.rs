use crate::service::signup_policy::SignupPolicyDenial;

use super::*;

fn config_values() -> serde_json::Value {
    let mut values = serde_json::json!({ "ENVIRONMENT": "local" });
    for key in [
        "BASE_URL",
        "DATABASE_URL",
        "REDIS_URI",
        "FUSIONAUTH_API_KEY_SECRET_KEY",
        "FUSIONAUTH_CLIENT_ID",
        "FUSIONAUTH_CLIENT_SECRET_KEY",
        "FUSIONAUTH_BASE_URL",
        "FUSIONAUTH_OAUTH_REDIRECT_URI",
        "GOOGLE_CLIENT_ID",
        "GOOGLE_CLIENT_SECRET_KEY",
        "STRIPE_SECRET_KEY",
        "SERVICE_INTERNAL_AUTH_KEY",
        "GITHUB_CLIENT_ID",
        "GITHUB_CLIENT_SECRET",
        "GITHUB_IDP_ID",
        "ACCOUNT_LINK_STATE_SECRET",
        "STRIPE_PRICE_ID",
        "INTERNAL_API_KEY",
        "KAFKA_BROKERS",
    ] {
        values[key] = serde_json::json!("test");
    }
    values["AI_USAGE_FREE_INCLUDED_ALLOWANCE_CENTS"] = serde_json::json!(500);
    values["AI_USAGE_INCLUDED_ALLOWANCE_CENTS"] = serde_json::json!(2_000);
    values["AI_USAGE_MAX_INCLUDED_ALLOWANCE_CENTS"] = serde_json::json!(10_000);
    values["AI_USAGE_OVERAGE_MARKUP_PERCENT"] = serde_json::json!(5);
    values
}

#[test]
fn ai_pricing_is_mandatory_and_validated() {
    use ai_billing::PlanTier;

    let config: Config = serde_json::from_value(config_values()).unwrap();
    let pricing = config.ai_pricing();
    assert_eq!(pricing.included_allowance_cents_for(PlanTier::Free), 500);
    assert_eq!(
        pricing.included_allowance_cents_for(PlanTier::Premium),
        2_000
    );
    assert_eq!(pricing.included_allowance_cents_for(PlanTier::Max), 10_000);
    assert_eq!(pricing.overage_markup_percent(), 5);

    for key in [
        "AI_USAGE_FREE_INCLUDED_ALLOWANCE_CENTS",
        "AI_USAGE_INCLUDED_ALLOWANCE_CENTS",
        "AI_USAGE_MAX_INCLUDED_ALLOWANCE_CENTS",
        "AI_USAGE_OVERAGE_MARKUP_PERCENT",
    ] {
        let mut values = config_values();
        values.as_object_mut().unwrap().remove(key);
        assert!(
            serde_json::from_value::<Config>(values.clone()).is_err(),
            "{key} must be mandatory"
        );
        values[key] = serde_json::json!("2000");
        assert!(
            serde_json::from_value::<Config>(values).is_err(),
            "{key} must be an integer"
        );
    }
    let mut values = config_values();
    values["AI_USAGE_OVERAGE_MARKUP_PERCENT"] = serde_json::json!(100);
    assert!(serde_json::from_value::<Config>(values).is_err());
    for key in [
        "AI_USAGE_FREE_INCLUDED_ALLOWANCE_CENTS",
        "AI_USAGE_INCLUDED_ALLOWANCE_CENTS",
        "AI_USAGE_MAX_INCLUDED_ALLOWANCE_CENTS",
    ] {
        let mut values = config_values();
        values[key] = serde_json::json!(-1);
        assert!(
            serde_json::from_value::<Config>(values).is_err(),
            "{key} must be non-negative"
        );
    }
}

#[test]
fn billing_enforcement_config_defaults_off_and_requires_a_boolean() {
    let mut values = config_values();
    let config: Config = serde_json::from_value(values.clone()).unwrap();
    assert_eq!(
        config.enable_ai_usage_enforcement,
        ai_billing::AiUsageEnforcement::Disabled
    );
    for enabled in [false, true] {
        values["ENABLE_AI_USAGE_ENFORCEMENT"] = serde_json::json!(enabled);
        let config: Config = serde_json::from_value(values.clone()).unwrap();
        assert_eq!(config.enable_ai_usage_enforcement.is_enabled(), enabled);
    }
    values["ENABLE_AI_USAGE_ENFORCEMENT"] = serde_json::json!("invalid");
    assert!(serde_json::from_value::<Config>(values).is_err());
}

#[test]
fn billing_settlement_config_defaults_off_and_requires_a_boolean() {
    let mut values = config_values();
    let config: Config = serde_json::from_value(values.clone()).unwrap();
    assert_eq!(
        config.enable_ai_usage_billing,
        ai_billing::AiUsageBilling::Disabled
    );
    for enabled in [false, true] {
        values["ENABLE_AI_USAGE_BILLING"] = serde_json::json!(enabled);
        let config: Config = serde_json::from_value(values.clone()).unwrap();
        assert_eq!(config.enable_ai_usage_billing.is_enabled(), enabled);
        // The two policies are independent.
        assert!(!config.enable_ai_usage_enforcement.is_enabled());
    }
    values["ENABLE_AI_USAGE_BILLING"] = serde_json::json!("invalid");
    assert!(serde_json::from_value::<Config>(values).is_err());
}

#[test]
fn account_link_state_secret_is_mandatory_and_must_be_long_enough() {
    let mut values = config_values();
    values
        .as_object_mut()
        .unwrap()
        .remove("ACCOUNT_LINK_STATE_SECRET");
    assert!(
        serde_json::from_value::<Config>(values).is_err(),
        "ACCOUNT_LINK_STATE_SECRET must be mandatory"
    );

    // The placeholder value every other key uses is far too short to sign with.
    let config: Config = serde_json::from_value(config_values()).unwrap();
    let error = config
        .validate_account_link_state_secret()
        .expect_err("a short signing secret must be rejected");
    assert!(error.to_string().contains("ACCOUNT_LINK_STATE_SECRET"));

    let mut values = config_values();
    values["ACCOUNT_LINK_STATE_SECRET"] = serde_json::json!("x".repeat(32));
    let config: Config = serde_json::from_value(values).unwrap();
    config.validate_account_link_state_secret().unwrap();
    config.account_link_state_key().unwrap();
}

fn development_allowlist(value: Option<&'static str>) -> DevelopmentSignupAllowlistJson {
    match value {
        Some(value) => DevelopmentSignupAllowlistJson::new_testing(value),
        None => DevelopmentSignupAllowlistJson::new_unset(),
    }
}

const ALLOWLIST_SETTING_CASES: [Option<&str>; 5] = [
    None,
    Some(""),
    Some(" \t "),
    Some("not-json-with-secret@example.test"),
    Some(r#"["allowed.user@example.test"]"#),
];

const UNLISTED_PUBLIC_EMAIL: &str = "unlisted.user@example.test";

#[test]
fn develop_config_temporarily_allows_unlisted_signups_regardless_of_bypass() {
    for bypass in [None, Some(false), Some(true)] {
        for allowlist in ALLOWLIST_SETTING_CASES {
            let mut values = config_values();
            values["ENVIRONMENT"] = serde_json::json!("develop");
            if let Some(bypass) = bypass {
                values["DEVELOPMENT_BYPASS_SIGNUP_ALLOWLIST"] = serde_json::json!(bypass);
            }
            if let Some(allowlist) = allowlist {
                values["DEVELOPMENT_SIGNUP_ALLOWLIST_JSON"] = serde_json::json!(allowlist);
            }
            let config: Config = serde_json::from_value(values).unwrap();

            // Exercise both runtime startup and the Doppler validator's entry point.
            for policy in [
                config.signup_policy().unwrap(),
                config
                    .signup_policy_for_environment(Environment::Develop)
                    .unwrap(),
            ] {
                assert_eq!(policy.allowed_email_count(), None);
                assert_eq!(policy.authorize_public_email(UNLISTED_PUBLIC_EMAIL), Ok(()));
            }
        }
    }
}

#[test]
fn develop_signup_policy_requires_configured_allowlist() {
    for value in [None, Some(""), Some(" \t ")] {
        let error =
            resolve_signup_policy(Environment::Develop, false, &development_allowlist(value))
                .expect_err("Develop should require a nonblank allowlist setting");

        assert!(
            error
                .to_string()
                .contains("DEVELOPMENT_SIGNUP_ALLOWLIST_JSON")
        );
    }
}

#[test]
fn develop_signup_policy_uses_configured_allowlist() {
    let policy = resolve_signup_policy(
        Environment::Develop,
        false,
        &development_allowlist(Some(
            r#"["Allowed.User@example.test", "allowed.user@example.test"]"#,
        )),
    )
    .expect("valid Develop allowlist should resolve");

    assert_eq!(policy.allowed_email_count(), Some(1));
    policy
        .authorize_public_email("allowed.user@example.test")
        .expect("configured email should be allowed");
    assert_eq!(
        policy.authorize_public_email(UNLISTED_PUBLIC_EMAIL),
        Err(SignupPolicyDenial::PublicEmailNotAllowed)
    );
}

#[test]
fn develop_signup_policy_rejects_malformed_allowlist_without_leaking_value() {
    let configured_value = "not-json-with-secret@example.test";
    let error = resolve_signup_policy(
        Environment::Develop,
        false,
        &development_allowlist(Some(configured_value)),
    )
    .expect_err("malformed Develop allowlist should be rejected");
    let message = error.to_string();

    assert!(message.contains("DEVELOPMENT_SIGNUP_ALLOWLIST_JSON"));
    assert!(!message.contains(configured_value));
    assert!(!format!("{error:?}").contains(configured_value));
}

#[test]
fn develop_signup_policy_bypass_ignores_allowlist_setting() {
    for value in ALLOWLIST_SETTING_CASES {
        let policy =
            resolve_signup_policy(Environment::Develop, true, &development_allowlist(value))
                .expect("Develop bypass should not parse the allowlist setting");

        assert_eq!(policy.allowed_email_count(), None);
        assert_eq!(policy.authorize_public_email(UNLISTED_PUBLIC_EMAIL), Ok(()));
    }
}

#[test]
fn production_and_local_signup_policy_ignore_allowlist_and_bypass() {
    for environment in [Environment::Production, Environment::Local] {
        for bypass in [false, true] {
            for value in ALLOWLIST_SETTING_CASES {
                let policy =
                    resolve_signup_policy(environment, bypass, &development_allowlist(value))
                        .expect("non-Develop environments should not parse the allowlist setting");

                assert_eq!(policy.allowed_email_count(), None);
                policy
                    .authorize_public_email("anyone@example.test")
                    .expect("non-Develop environments should allow all public signups");
            }
        }
    }
}
