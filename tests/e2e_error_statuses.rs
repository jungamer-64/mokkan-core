use axum::body::Body;
use axum::http::{Request, header::AUTHORIZATION, Method, StatusCode};
use tower::util::ServiceExt as _;

mod support;

/// 存在しないスラグで 404 Not Found を返すことを確認する
#[tokio::test]
async fn e2e_get_article_by_slug_not_found_returns_404() {
    let app = support::make_test_router().await;

    let req = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/articles/by-slug/nonexistent")
        .header(AUTHORIZATION, "Bearer test-token")
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    support::assert_error_response(resp, StatusCode::NOT_FOUND, "Not Found").await;
}

/// 権限がない操作で 403 Forbidden を返すことを確認する
#[tokio::test]
async fn e2e_create_article_forbidden_returns_403() {
    let app = support::make_test_router().await;

    let body = serde_json::json!({ "title": "t", "body": "b", "publish": false }).to_string();
    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/articles")
        .header(AUTHORIZATION, "Bearer test-token")
        .header("content-type", "application/json")
        .body(Body::from(body))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    support::assert_error_response(resp, StatusCode::FORBIDDEN, "Forbidden").await;
}
