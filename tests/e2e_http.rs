// tests/e2e_http.rs
use axum::body::{self, Body};
use axum::http::{Request, StatusCode, header::AUTHORIZATION};
use serde_json::Value;
use tower::util::ServiceExt as _;

mod support;

/// 無効トークンで 401 Unauthorized を返すことを確認する
#[tokio::test]
async fn e2e_audit_list_endpoint_invalid_token_returns_401() {
    let app = support::make_test_router().await;

    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/audit-logs")
        .header(AUTHORIZATION, "Bearer bad-token")
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_error_response_async!(resp, StatusCode::UNAUTHORIZED, "Unauthorized").await;
}

/// 簡易E2E: /healthおよび /api/v1/audit-logs が200を返すことを確認する
#[tokio::test]
async fn e2e_audit_list_endpoint_returns_200() {
    let app = support::make_test_router().await;

    // sanity check: /health should return 200
    let health_req = Request::builder()
        .method("GET")
        .uri("/health")
        .body(Body::empty())
        .unwrap();
    let health_resp = app.clone().oneshot(health_req).await.unwrap();
    let health_status = health_resp.status();
    let (health_parts, health_body_stream) = health_resp.into_parts();
    let health_body_bytes = body::to_bytes(health_body_stream, 1024 * 1024)
        .await
        .unwrap();
    // assert Content-Type for health starts with application/json
    let health_ct = health_parts
        .headers
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(
        health_ct.starts_with("application/json"),
        "unexpected content-type: {}",
        health_ct
    );
    if health_status != StatusCode::OK {
        let s = String::from_utf8_lossy(&health_body_bytes);
        panic!("health endpoint expected 200, got {}: {}", health_status, s);
    }

    // Directly call the handler to confirm it works without router layers
    let direct = mokkan_core::presentation::http::routes::health().await;
    assert_eq!(direct.0.status, "ok");

    let req = Request::builder()
        .method("GET")
        .uri("/api/v1/audit-logs")
        .header(AUTHORIZATION, "Bearer test-token")
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    let status = resp.status();
    let (parts, body_stream) = resp.into_parts();
    let body_bytes = body::to_bytes(body_stream, 1024 * 1024).await.unwrap();
    // assert Content-Type starts with application/json
    let ct = parts
        .headers
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(
        ct.starts_with("application/json"),
        "unexpected content-type: {}",
        ct
    );
    if status != StatusCode::OK {
        let s = String::from_utf8_lossy(&body_bytes);
        panic!("expected 200 OK, got {}: {}", status, s);
    }
    let json: Value = serde_json::from_slice(&body_bytes).unwrap();
    // expect items array with at least one element
    assert!(
        json.get("items")
            .and_then(|v| v.as_array())
            .map(|a| a.len() >= 1)
            .unwrap_or(false)
    );
}
