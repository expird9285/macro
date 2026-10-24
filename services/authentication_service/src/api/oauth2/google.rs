use axum::response::Response;
use tower_cookies::Cookies;

use crate::api::{
    context::ApiContext,
    oauth2::{OAuthState, login},
};

/// Completes a Google sign-in. Account links are GitHub-only, so a verified
/// `link_id` state never reaches the Google callback.
pub(in crate::api::oauth2) async fn handler(
    ctx: &ApiContext,
    cookies: Cookies,
    code: &str,
    state: &OAuthState,
) -> Result<Response, Response> {
    login::handler(ctx, cookies, code, "google", state).await
}
