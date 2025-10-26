use axum::body::Body;
use axum::http::{Request, header::AUTHORIZATION, Method, StatusCode};
use tower::util::ServiceExt as _;
use sha2::{Digest, Sha256};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use serde_urlencoded;

mod support;

fn bearer(tok: &str) -> String { format!("Bearer {}", tok) }

#[tokio::test]
async fn authorize_code_flow_pkce_plain() {
    let app = support::make_test_router().await;

    // Request an authorization code (no redirect_uri -> code returned in JSON)
    let uri = "/api/v1/auth/authorize?response_type=code&code_challenge=verifier&code_challenge_method=plain&consent=approve";
    let req = Request::builder()
        .method(Method::GET)
        .uri(uri)
        .header(AUTHORIZATION, bearer(support::TEST_TOKEN))
        .body(Body::empty())
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let (_h, json) = to_json_async!(resp).await;
    let code = json.get("code").and_then(|v| v.as_str()).expect("code present");

    // Exchange the code for tokens using PKCE (plain verifier)
    let body = serde_urlencoded::to_string(&[
        ("grant_type", "authorization_code"),
        ("code", code),
        ("code_verifier", "verifier"),
    ])
    .unwrap();

    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/auth/token")
        .header("content-type", "application/x-www-form-urlencoded")
        .body(Body::from(body))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let (_h, json) = to_json_async!(resp).await;
    let token = json.get("token").and_then(|v| v.as_str()).expect("token present");

    // DummyTokenManager.issue returns issued-<user_id> for our test user (user id 1)
    assert_eq!(token, "issued-1");
}


#[tokio::test]
async fn authorize_code_flow_pkce_s256() {
    let app = support::make_test_router().await;

    // Use a verifier and compute S256 challenge
    let verifier = "some-long-random-verifier";
    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    let digest = hasher.finalize();
    let challenge = URL_SAFE_NO_PAD.encode(&digest[..]);

    // Request an authorization code (no redirect_uri -> code returned in JSON)
    let uri = format!(
        "/api/v1/auth/authorize?response_type=code&code_challenge={}&code_challenge_method=S256&consent=approve",
        challenge
    );

    let req = Request::builder()
        .method(Method::GET)
        .uri(uri)
        .header(AUTHORIZATION, bearer(support::TEST_TOKEN))
        .body(Body::empty())
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let (_h, json) = to_json_async!(resp).await;
    let code = json.get("code").and_then(|v| v.as_str()).expect("code present");

    // Exchange the code for tokens using PKCE S256
    let body = serde_urlencoded::to_string(&[
        ("grant_type", "authorization_code"),
        ("code", code),
        ("code_verifier", verifier),
    ])
    .unwrap();

    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/auth/token")
        .header("content-type", "application/x-www-form-urlencoded")
        .body(Body::from(body))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let (_h, json) = to_json_async!(resp).await;
    let token = json.get("token").and_then(|v| v.as_str()).expect("token present");
    assert_eq!(token, "issued-1");
}
