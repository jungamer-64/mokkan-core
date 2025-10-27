// src/presentation/http/openapi/tests.rs
use super::*;
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};

#[test]
fn weak_match_handles_strong_and_weak_tags() {
    assert!(weak_match(r#"W/\"abc\""#, r#"\"abc\""#));
    assert!(weak_match(r#"\"abc\""#, r#"W/\"abc\""#));
    assert!(!weak_match(r#"\"abc\""#, r#"\"def\""#));
}

#[test]
fn inm_matches_star_header() {
    let mut headers = HeaderMap::new();
    headers.insert(header::IF_NONE_MATCH, HeaderValue::from_static("*"));
    assert!(inm_matches(&headers, r#"\"anything\""#));
}

#[test]
fn inm_matches_comma_separated_values() {
    let mut headers = HeaderMap::new();
    headers.insert(
        header::IF_NONE_MATCH,
        HeaderValue::from_static("\"foo\", W/\"bar\""),
    );
    let header_str = headers
        .get(header::IF_NONE_MATCH)
        .unwrap()
        .to_str()
        .unwrap();
    let candidates: Vec<&str> = header_str.split(',').map(str::trim).collect();
    assert!(weak_match(candidates[1], r#"W/\"bar\""#));
    assert!(inm_matches(&headers, r#"W/\"bar\""#));
    assert_eq!(candidates, vec!["\"foo\"", "W/\"bar\""]);
}

#[test]
fn weak_match_handles_lowercase_prefix() {
    assert!(weak_match(r#"w/\"abc\""#, r#"\"abc\""#));
    assert!(weak_match(r#"\"abc\""#, r#"w/\"abc\""#));
}

#[tokio::test]
async fn serve_openapi_returns_not_modified_when_if_none_match_matches() {
    // build headers that match current etag
    let mut headers = HeaderMap::new();
    headers.insert(
        header::IF_NONE_MATCH,
        HeaderValue::from_static(openapi_etag()),
    );

    let resp = serve_openapi(headers).await;
    assert_eq!(resp.status(), StatusCode::NOT_MODIFIED);
}

#[tokio::test]
async fn head_openapi_ok_sets_headers_and_no_body() {
    let headers = HeaderMap::new();
    let resp = head_openapi(headers).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let hs = resp.headers();
    assert!(hs.get(header::ETAG).is_some());
    assert!(hs.get(header::CONTENT_TYPE).is_some());
    assert!(hs.get(header::CONTENT_LENGTH).is_some());
    // HEAD so body must be empty — Content-Length should equal the OpenAPI length
    let cl = hs.get(header::CONTENT_LENGTH).unwrap().to_str().unwrap();
    assert_eq!(cl, openapi_content_length().to_string());
}

#[tokio::test]
async fn head_openapi_returns_not_modified_when_if_none_match_matches() {
    let mut headers = HeaderMap::new();
    headers.insert(
        header::IF_NONE_MATCH,
        HeaderValue::from_static(openapi_etag()),
    );
    let resp = head_openapi(headers).await;
    assert_eq!(resp.status(), StatusCode::NOT_MODIFIED);
    assert!(resp.headers().get(header::ETAG).is_some());
}

#[allow(clippy::option_env_unwrap)]
#[tokio::test]
async fn get_openapi_returns_not_modified_on_ims() {
    // Skip if BUILD_DATE not embedded during build
    if option_env!("BUILD_DATE").is_none() {
        return;
    }
    let mut headers = HeaderMap::new();
    headers.insert(
        header::IF_MODIFIED_SINCE,
        HeaderValue::from_static(option_env!("BUILD_DATE").unwrap()),
    );
    let resp = serve_openapi(headers).await;
    assert_eq!(resp.status(), StatusCode::NOT_MODIFIED);
}

#[tokio::test]
async fn get_openapi_inm_takes_precedence_over_ims() {
    // If both INM and IMS are present, INM must take precedence per RFC.
    let mut headers = HeaderMap::new();
    headers.insert(
        header::IF_NONE_MATCH,
        HeaderValue::from_static(openapi_etag()),
    );
    headers.insert(
        header::IF_MODIFIED_SINCE,
        HeaderValue::from_static("Thu, 01 Jan 1970 00:00:00 GMT"),
    );
    let resp = serve_openapi(headers).await;
    assert_eq!(resp.status(), StatusCode::NOT_MODIFIED);
}

#[allow(clippy::option_env_unwrap)]
#[tokio::test]
async fn get_openapi_returns_ok_when_inm_mismatch_even_if_ims_matches() {
    // Only meaningful when BUILD_DATE is present (we compare against it)
    if option_env!("BUILD_DATE").is_none() {
        return;
    }
    let lm = option_env!("BUILD_DATE").unwrap();
    let mut headers = HeaderMap::new();
    // intentionally mismatching ETag
    headers.insert(
        header::IF_NONE_MATCH,
        HeaderValue::from_static("\"some-other\""),
    );
    headers.insert(header::IF_MODIFIED_SINCE, HeaderValue::from_static(lm));
    let resp = serve_openapi(headers).await;
    assert_eq!(resp.status(), StatusCode::OK);
}
