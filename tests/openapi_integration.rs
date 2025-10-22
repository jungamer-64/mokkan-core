use axum::body::Body;
use axum::http::{Method, Request, header};
use mokkan_core::presentation::http::openapi::docs_router_with_options;
use tower::ServiceExt; // for oneshot

#[tokio::test]
async fn docs_router_get_openapi_json_returns_ok_and_etag() {
    let app = docs_router_with_options(true, false);

    let req = Request::builder()
        .method(Method::GET)
        .uri("/openapi.json")
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), 200);
    assert!(resp.headers().get(header::ETAG).is_some());
}

#[tokio::test]
async fn docs_router_head_with_if_none_match_returns_304() {
    let app = docs_router_with_options(true, false);
    // first get to obtain etag
    let req = Request::builder()
        .method(Method::GET)
        .uri("/openapi.json")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let etag = resp
        .headers()
        .get(header::ETAG)
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();

    // now HEAD with If-None-Match
    let head_req = Request::builder()
        .method(Method::HEAD)
        .uri("/openapi.json")
        .header(header::IF_NONE_MATCH, etag.as_str())
        .body(Body::empty())
        .unwrap();

    let head_resp = app.oneshot(head_req).await.unwrap();
    assert_eq!(head_resp.status(), 304);
    // Content-Length may be omitted; if present it must be "0"
    if let Some(cl) = head_resp.headers().get(header::CONTENT_LENGTH) {
        assert_eq!(cl.to_str().unwrap(), "0");
    }
}
