// tests/e2e_rbac.rs
use axum::body::Body;
use axum::http::{Request, header::AUTHORIZATION, Method, StatusCode};
use tower::util::ServiceExt as _;

mod support;

fn bearer(tok: &str) -> String { format!("Bearer {}", tok) }

#[tokio::test]
async fn grant_role_forbidden_without_capability() {
    let app = support::make_test_router().await;

    let body = serde_json::json!({ "role": "admin" }).to_string();
    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/users/1/grant-role")
        .header(AUTHORIZATION, bearer(support::NO_AUDIT_TOKEN))
        .header("content-type", "application/json")
        .body(Body::from(body))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_error_response_async!(resp, StatusCode::FORBIDDEN, "Forbidden").await;
}

#[tokio::test]
async fn grant_role_with_admin_token_returns_not_found_for_dummy_repo() {
    let app = support::make_test_router().await;

    let body = serde_json::json!({ "role": "admin" }).to_string();
    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/users/1/grant-role")
        .header(AUTHORIZATION, bearer(support::TEST_TOKEN))
        .header("content-type", "application/json")
        .body(Body::from(body))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_error_response_async!(resp, StatusCode::BAD_REQUEST, "Bad Request").await;
}

#[tokio::test]
async fn revoke_role_forbidden_without_capability() {
    let app = support::make_test_router().await;

    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/users/1/revoke-role")
        .header(AUTHORIZATION, bearer(support::NO_AUDIT_TOKEN))
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_error_response_async!(resp, StatusCode::FORBIDDEN, "Forbidden").await;
}

#[tokio::test]
async fn revoke_role_with_admin_token_returns_not_found_for_dummy_repo() {
    let app = support::make_test_router().await;

    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/users/1/revoke-role")
        .header(AUTHORIZATION, bearer(support::TEST_TOKEN))
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_error_response_async!(resp, StatusCode::BAD_REQUEST, "Bad Request").await;
}
