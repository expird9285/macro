use axum::http::{StatusCode, header::LOCATION};
use url::Url;

use super::build_return_redirect;

#[test]
fn return_redirect_replaces_a_previous_outcome_and_keeps_the_rest() {
    let original_url = urlencoding::encode(
        "macro:///link-callback?link_error=cancelled&provider=github&token=stale&view=all",
    );

    let response = build_return_redirect(
        &original_url,
        &[
            ("link_id", "694d20f7-ff85-46fd-b17d-5f68ff2d3707"),
            ("link_error", "failed"),
            ("provider", "github"),
        ],
    )
    .unwrap();
    let redirect_url = Url::parse(response.headers()[LOCATION].to_str().unwrap()).unwrap();
    let query_pairs: Vec<_> = redirect_url.query_pairs().collect();

    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(redirect_url.scheme(), "macro");
    assert_eq!(
        query_pairs,
        vec![
            ("view".into(), "all".into()),
            (
                "link_id".into(),
                "694d20f7-ff85-46fd-b17d-5f68ff2d3707".into()
            ),
            ("link_error".into(), "failed".into()),
            ("provider".into(), "github".into()),
        ]
    );
}
