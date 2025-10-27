use axum::body::Body;
use axum::http::{Method, Request, StatusCode, header::AUTHORIZATION};
use tower::util::ServiceExt as _;

mod support;

use support::mocks::SESSION_TOKEN;

#[tokio::test]
async fn logout_revokes_session_and_protects_endpoints() {
    let app = support::make_test_router().await;

    // Precondition: session token can access protected endpoint (create article) -> not 401.
    let body = serde_json::json!({ "title": "t", "body": "b", "publish": false }).to_string();
    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/articles")
        .header(AUTHORIZATION, format!("Bearer {}", SESSION_TOKEN))
        .header("content-type", "application/json")
        .body(Body::from(body))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    // The dummy article write repo returns NotFound; ensure we reached the handler (not 401/403).
    assert!(resp.status() != StatusCode::UNAUTHORIZED && resp.status() != StatusCode::FORBIDDEN);

    // Call logout
    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/auth/logout")
        .header(AUTHORIZATION, format!("Bearer {}", SESSION_TOKEN))
        .body(Body::empty())
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let (_headers, json) = to_json_async!(resp).await;
    assert_eq!(
        json.get("status").and_then(|v| v.as_str()),
        Some("logged_out")
    );

    // After logout the protected endpoint should now be unauthorized (401)
    let body = serde_json::json!({ "title": "t2", "body": "b2", "publish": false }).to_string();
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
