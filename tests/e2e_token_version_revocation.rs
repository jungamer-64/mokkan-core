use axum::body::Body;
use axum::http::{Method, Request, StatusCode, header::AUTHORIZATION};
use tower::util::ServiceExt as _;

mod support;

use support::mocks::SESSION_TOKEN;

#[tokio::test]
async fn token_version_revocation_honors_min_version() {
    // Build state so we can set min token version
    let state = support::build_test_state().await;

    // user id for session_user is 4; set min token version to 2
    state
        .services
        .session_revocation_store()
        .set_min_token_version(4, 2)
        .await
        .expect("set min version");

    let app = mokkan_core::presentation::http::routes::build_router_with_rate_limiter(state, false);

    // The token has token_version = 1, so it should now be rejected
    let body = serde_json::json!({ "title": "t", "body": "b", "publish": false }).to_string();
    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/articles")
        .header(AUTHORIZATION, format!("Bearer {}", SESSION_TOKEN))
        .header("content-type", "application/json")
        .body(Body::from(body))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_error_response_async!(resp, StatusCode::UNAUTHORIZED, "Unauthorized").await;
}
