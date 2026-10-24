use crate::api::context::ApiContext;
use axum::{
    Router,
    routing::{delete, get, post},
};
pub(in crate::api) mod create_in_progress_link;
pub(in crate::api) mod github;

/// Routes for authenticated account-link operations.
pub fn router() -> Router<ApiContext> {
    Router::new()
        .route("/", post(create_in_progress_link::handler))
        .route("/github", post(github::init_github_link_handler))
        .route("/github", delete(github::delete_github_link_handler))
        .route(
            "/github/status",
            get(github::check_github_link_status_handler),
        )
}
