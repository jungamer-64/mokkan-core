use axum::body::Body;
use axum::http::{Request, Method, StatusCode};
use tower::util::ServiceExt as _;

mod support;

#[tokio::test]
async fn openid_discovery_returns_document() {
    let app = support::make_test_router().await;

    let req = Request::builder()
        .method(Method::GET)
        .uri("/.well-known/openid-configuration")
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let (_headers, json) = to_json_async!(resp).await;

    let issuer = json.get("issuer").and_then(|v| v.as_str()).expect("issuer present");
    let jwks = json.get("jwks_uri").and_then(|v| v.as_str()).expect("jwks_uri present");

    // Basic checks
    assert!(issuer.starts_with("http"), "issuer should be an URL");
    assert!(jwks.ends_with("/api/v1/auth/keys"), "jwks uri should point to keys endpoint");

    // Check userinfo endpoint mapping
    let userinfo = json.get("userinfo_endpoint").and_then(|v| v.as_str()).expect("userinfo present");
    assert!(userinfo.ends_with("/api/v1/auth/me"), "userinfo endpoint should point to /api/v1/auth/me");

    // Ensure claims_supported contains 'sub'
    let claims = json.get("claims_supported").and_then(|v| v.as_array()).expect("claims present");
    assert!(claims.iter().any(|c| c.as_str() == Some("sub")), "claims_supported should include 'sub'");

    // Ensure grant_types contains authorization_code
    let grants = json.get("grant_types_supported").and_then(|v| v.as_array()).expect("grant_types present");
    assert!(grants.iter().any(|g| g.as_str() == Some("authorization_code")), "grant_types_supported should include 'authorization_code'");
}
