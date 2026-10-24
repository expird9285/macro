use super::*;

#[test]
fn callback_original_url_allows_trusted_destinations() {
    for original_url in [
        "macro://login",
        "tauri://localhost/app/login",
        "http://localhost/app/login",
        "https://dev.macro.com/app/login",
        "https://macro.com/app/login",
        "https://www.macro.com/app/link-callback",
    ] {
        assert!(
            validate_original_url(Some(original_url)).is_ok(),
            "{original_url} should be allowed"
        );
    }
}

#[test]
fn callback_original_url_rejects_untrusted_destinations() {
    for original_url in [
        "https://evil.example.com/phish",
        "https://macro.com.example.com/phish",
        "https://www.macro.com.example.com/phish",
        "http://macro.com/phish",
        "http://www.macro.com/phish",
        "javascript:alert('redirected')",
    ] {
        assert!(
            matches!(
                validate_original_url(Some(original_url)),
                Err(OriginalUrlValidationError::Disallowed(_))
            ),
            "{original_url} should be rejected"
        );
    }
}

#[test]
fn callback_original_url_validates_decoded_destination() {
    let trusted_url = urlencoding::encode("https://www.macro.com/app/link-callback");
    assert!(validate_original_url(Some(&trusted_url)).is_ok());

    let encoded_url = urlencoding::encode("https://evil.example.com/phish");

    assert!(matches!(
        validate_original_url(Some(&encoded_url)),
        Err(OriginalUrlValidationError::Disallowed(_))
    ));
}

#[test]
fn callback_original_url_rejects_invalid_urls() {
    assert!(matches!(
        validate_original_url(Some("%ZZ")),
        Err(OriginalUrlValidationError::Invalid)
    ));
    assert!(validate_original_url(None).is_ok());
}

use crate::account_link_state::{AccountLinkState, AccountLinkStateKey, sign_account_link_state};
use uuid::Uuid;

const NOW: i64 = 1_730_000_000;

fn key() -> AccountLinkStateKey {
    AccountLinkStateKey::new("oauth2 callback state test secret value").unwrap()
}

fn link_state(provider: LinkProvider, exp: i64) -> AccountLinkState {
    AccountLinkState {
        provider,
        identity_provider_id: "idp-for-linking".into(),
        link_id: Uuid::parse_str("0192b8a0-6a2e-7c3e-9b1f-4d5e6f708192").unwrap(),
        fusion_user_id: Uuid::parse_str("8c06ab3e-693c-45a7-8f92-1b5a5bf876ac").unwrap(),
        original_url: Some("https://macro.com/app/link-callback".into()),
        exp,
    }
}

#[test]
fn callback_accepts_signed_state_issued_for_its_provider() {
    let provider = LinkProvider::Github;
    let state = link_state(provider, NOW + 60);
    let token = sign_account_link_state(&state, &key()).unwrap();

    match parse_callback_state(&token, provider.as_str(), &key(), NOW).unwrap() {
        CallbackState::AccountLink(verified) => assert_eq!(verified, state),
        other => panic!("expected an account link for {provider}, got {other:?}"),
    }
}

#[test]
fn callback_rejects_signed_state_issued_for_another_provider() {
    let token =
        sign_account_link_state(&link_state(LinkProvider::Github, NOW + 60), &key()).unwrap();

    assert_eq!(
        parse_callback_state(&token, "google", &key(), NOW).unwrap_err(),
        CallbackStateError::ProviderMismatch {
            issued_for: LinkProvider::Github
        }
    );
}

#[test]
fn callback_rejects_expired_foreign_and_garbage_tokens() {
    let state = link_state(LinkProvider::Github, NOW);
    let expired = sign_account_link_state(&state, &key()).unwrap();
    let other_key = AccountLinkStateKey::new("a different signing key of enough length").unwrap();
    let foreign =
        sign_account_link_state(&link_state(LinkProvider::Github, NOW + 60), &other_key).unwrap();

    assert_eq!(
        parse_callback_state(&expired, "github", &key(), NOW).unwrap_err(),
        CallbackStateError::Signed(AccountLinkStateError::Expired {
            link_id: state.link_id
        })
    );
    assert_eq!(
        parse_callback_state(&foreign, "github", &key(), NOW).unwrap_err(),
        CallbackStateError::Signed(AccountLinkStateError::InvalidSignature)
    );
    assert_eq!(
        parse_callback_state("not-a-token", "github", &key(), NOW).unwrap_err(),
        CallbackStateError::Signed(AccountLinkStateError::Malformed)
    );
}

#[test]
fn callback_accepts_unsigned_state_only_for_sign_in() {
    let login = parse_callback_state(
        r#"{"identity_provider_id":"google-login","is_mobile":true}"#,
        "google",
        &key(),
        NOW,
    )
    .unwrap();
    match login {
        CallbackState::Login(state) => {
            assert_eq!(state.identity_provider_id, "google-login");
            assert_eq!(state.is_mobile, Some(true));
            assert!(state.link_id.is_none());
            assert!(state.original_url.is_none());
        }
        other => panic!("expected sign-in state, got {other:?}"),
    }

    // The pre-signing wire format: an attacker-chosen link_id in plain JSON.
    let forged = format!(
        r#"{{"identity_provider_id":"github-link","link_id":"{}","original_url":"https://macro.com/app"}}"#,
        Uuid::now_v7()
    );
    assert_eq!(
        parse_callback_state(&forged, "google", &key(), NOW).unwrap_err(),
        CallbackStateError::UnsignedAccountLink
    );
    assert_eq!(
        parse_callback_state("{not json", "google", &key(), NOW).unwrap_err(),
        CallbackStateError::Unparseable
    );
}

#[test]
fn consent_failures_are_classified_from_provider_errors() {
    for (error, expected) in [
        (Some("access_denied"), ConsentFailure::Cancelled),
        (Some("server_error"), ConsentFailure::Failed),
        (Some("invalid_request"), ConsentFailure::Failed),
        (None, ConsentFailure::Failed),
    ] {
        assert_eq!(ConsentFailure::classify(error), expected, "{error:?}");
    }
}

fn redirect_query(response: Response) -> Vec<(String, String)> {
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    let location = response.headers()[axum::http::header::LOCATION]
        .to_str()
        .unwrap()
        .to_owned();
    Url::parse(&location)
        .unwrap()
        .query_pairs()
        .into_owned()
        .collect()
}

fn params(query: &str) -> Params {
    serde_urlencoded::from_str(query).unwrap()
}

#[test]
fn consent_failure_sends_a_link_flow_back_to_the_app() {
    let link = link_state(LinkProvider::Github, NOW + 60);

    let response = consent_failure_response(Some(&link), &params("state=s&error=access_denied"));

    assert_eq!(
        redirect_query(response),
        vec![
            ("link_id".into(), link.link_id.to_string()),
            ("link_error".into(), "cancelled".into()),
            ("provider".into(), "github".into()),
        ]
    );
}

#[test]
fn consent_failure_during_sign_in_keeps_the_error_body() {
    let response = consent_failure_response(None, &params("state=s&error=access_denied"));

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[test]
fn verified_link_state_converts_to_provider_handler_state() {
    let link = link_state(LinkProvider::Github, NOW + 60);

    let state = OAuthState::from(&link);

    assert_eq!(state.identity_provider_id, link.identity_provider_id);
    assert_eq!(state.link_id, Some(link.link_id));
    assert_eq!(state.original_url, link.original_url);
    assert_eq!(state.is_mobile, None);
}
