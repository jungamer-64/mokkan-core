// tests/e2e_auth_and_cursor.rs
use axum::body::Body;
use axum::http::{Request, header::AUTHORIZATION, StatusCode};
use tower::util::ServiceExt as _;
use serde_json::Value;

mod support;

#[tokio::test]
async fn missing_token_returns_401() {
    let app = support::make_test_router().await;
    let req = Request::builder().method("GET").uri("/api/v1/audit-logs").body(Body::empty()).unwrap();
    let resp = app.oneshot(req).await.unwrap();
    support::assert_error_response(resp, StatusCode::UNAUTHORIZED, "Unauthorized").await;
}

#[tokio::test]
async fn capability_failure_returns_403() {
    let app = support::make_test_router().await;
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/audit-logs")
        .header(AUTHORIZATION, "Bearer no-audit")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    support::assert_error_response(resp, StatusCode::FORBIDDEN, "Forbidden").await;
}

#[tokio::test]
async fn expired_token_returns_401() {
    let app = support::make_test_router().await;
    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/audit-logs")
        .header(AUTHORIZATION, "Bearer expired-token")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    support::assert_error_response(resp, StatusCode::UNAUTHORIZED, "Unauthorized").await;
}

#[tokio::test]
async fn invalid_cursor_returns_400() {
    // Inject a deterministic audit repo but request an invalid cursor
    let audit_repo = std::sync::Arc::new(support::MockRepo { items: vec![], next_cursor: None });
    let app = support::make_test_router_with_audit_repo(audit_repo).await;
    let uri = "/api/v1/audit-logs?cursor=not-a-valid-cursor";
    let req = Request::builder().method("GET").uri(uri).header(AUTHORIZATION, "Bearer test-token").body(Body::empty()).unwrap();
    let resp = app.oneshot(req).await.unwrap();
    support::assert_error_response(resp, StatusCode::BAD_REQUEST, "Bad Request").await;
}

#[tokio::test]
async fn next_cursor_propagates_in_response() {
    // Use MockRepo that returns next_cursor
    let mut repo = support::MockRepo { items: vec![], next_cursor: Some("next-123".into()) };
    let audit_repo = std::sync::Arc::new(repo);
    let app = support::make_test_router_with_audit_repo(audit_repo).await;
    let req = Request::builder().method("GET").uri("/api/v1/audit-logs").header(AUTHORIZATION, "Bearer test-token").body(Body::empty()).unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let (parts, body_stream) = resp.into_parts();
    let bytes = axum::body::to_bytes(body_stream, 1024 * 1024).await.unwrap();
    let ct = parts.headers.get("content-type").and_then(|v| v.to_str().ok()).unwrap_or("");
    assert!(ct.starts_with("application/json"));
    let json: Value = serde_json::from_slice(&bytes).unwrap();
    let next = json.get("next_cursor").and_then(|v| v.as_str()).unwrap_or("");
    assert_eq!(next, "next-123");
}
