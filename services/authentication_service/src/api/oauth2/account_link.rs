use axum::{
    Json,
    response::{IntoResponse, Redirect, Response},
};
use model::response::ErrorResponse;
use reqwest::StatusCode;
use url::Url;
use uuid::Uuid;

use crate::api::context::ApiContext;

#[cfg(test)]
mod test;

#[derive(Debug)]
pub(super) enum CallbackRedirectError {
    UnableToDecode,
    UnableToParse,
}

impl IntoResponse for CallbackRedirectError {
    fn into_response(self) -> Response {
        let message = match self {
            Self::UnableToDecode => "unable to decode original url",
            Self::UnableToParse => "unable to parse to original url",
        };

        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                message: message.into(),
            }),
        )
            .into_response()
    }
}

/// Query keys the callback owns on the return URL. Replaced, never duplicated,
/// so a return URL that already carries a previous outcome does not keep it.
const CALLBACK_QUERY_KEYS: [&str; 4] = ["link_id", "token", "link_error", "provider"];

/// Builds the redirect back to the caller's return URL carrying `outcome`
/// query pairs, such as a consent error.
pub(super) fn build_return_redirect(
    original_url: &str,
    outcome: &[(&str, &str)],
) -> Result<Response, CallbackRedirectError> {
    let decoded_url = urlencoding::decode(original_url).map_err(|error| {
        tracing::error!(error=?error, "unable to decode original url");
        CallbackRedirectError::UnableToDecode
    })?;

    let mut url: Url = decoded_url
        .parse()
        .inspect_err(|error| tracing::error!(error=?error, "unable to parse string to url"))
        .map_err(|_| CallbackRedirectError::UnableToParse)?;

    let preserved_query_pairs: Vec<(String, String)> = url
        .query_pairs()
        .filter(|(key, _)| !CALLBACK_QUERY_KEYS.contains(&key.as_ref()))
        .map(|(key, value)| (key.into_owned(), value.into_owned()))
        .collect();
    url.query_pairs_mut()
        .clear()
        .extend_pairs(preserved_query_pairs)
        .extend_pairs(outcome);

    Ok(Redirect::to(url.as_str()).into_response())
}

/// Removes a pending account-link row after a callback failure.
pub(super) async fn cleanup_pending_link(ctx: &ApiContext, link_id: &Uuid) {
    macro_db_client::in_progress_user_link::delete_in_progress_user_link(&ctx.db, link_id)
        .await
        .inspect_err(|error| {
            tracing::warn!(
                error=?error,
                ?link_id,
                "failed to clean up in_progress_user_link after account-link callback error"
            );
        })
        .ok();
}
