// tests/e2e_auth_and_cursor.rs
use axum::body::Body;
use axum::http::{
    Request, StatusCode,
    header::{AUTHORIZATION, CONTENT_TYPE},
};

const AUDIT: &str = "/api/v1/audit-logs";
fn bearer(tok: &str) -> String {
    format!("Bearer {}", tok)
}
use tower::util::ServiceExt as _;

mod support;

#[tokio::test]
async fn missing_token_returns_401() {
    let app = support::make_test_router().await;
    let req = Request::builder()
        .method("GET")
        .uri(AUDIT)
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_error_response_async!(resp, StatusCode::UNAUTHORIZED, "Unauthorized").await;
}

#[tokio::test]
async fn capability_failure_returns_403() {
    let app = support::make_test_router().await;
    let req = Request::builder()
        .method("GET")
        .uri(AUDIT)
        .header(AUTHORIZATION, bearer(support::NO_AUDIT_TOKEN))
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_error_response_async!(resp, StatusCode::FORBIDDEN, "Forbidden").await;
}

#[tokio::test]
async fn expired_token_returns_401() {
    let app = support::make_test_router().await;
    let req = Request::builder()
        .method("GET")
        .uri(AUDIT)
        .header(AUTHORIZATION, bearer(support::EXPIRED_TOKEN))
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_error_response_async!(resp, StatusCode::UNAUTHORIZED, "Unauthorized").await;
}

#[tokio::test]
async fn invalid_cursor_returns_400() {
    // Inject a deterministic audit repo but request an invalid cursor
    let audit_repo = std::sync::Arc::new(support::MockRepo {
        items: vec![],
        next_cursor: None,
    });
    let app = support::make_test_router_with_audit_repo(audit_repo).await;
    let uri = format!("{AUDIT}?cursor=not-a-valid-cursor");
    let req = Request::builder()
        .method("GET")
        .uri(&uri)
        .header(AUTHORIZATION, bearer(support::TEST_TOKEN))
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_error_response_async!(resp, StatusCode::BAD_REQUEST, "Bad Request").await;
}

#[tokio::test]
async fn next_cursor_propagates_in_response() {
    // Use MockRepo that returns next_cursor
    let repo = support::MockRepo {
        items: vec![],
        next_cursor: Some("next-123".into()),
    };
    let audit_repo = std::sync::Arc::new(repo);
    let app = support::make_test_router_with_audit_repo(audit_repo).await;
    let req = Request::builder()
        .method("GET")
        .uri(AUDIT)
        .header(AUTHORIZATION, bearer(support::TEST_TOKEN))
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let (headers, json) = to_json_async!(resp).await;
    let ct = headers
        .get(CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(ct.starts_with("application/json"));
    assert_eq!(
        json.get("next_cursor")
            .and_then(|v| v.as_str())
            .unwrap_or(""),
        "next-123"
    );
}
