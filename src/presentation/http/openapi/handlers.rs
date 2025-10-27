use axum::{
    body::Body,
    http::{HeaderMap, StatusCode, header},
    response::Response,
};

/// Build a 304 Not Modified response including the current ETag header.
fn not_modified_response() -> Response {
    let mut b = Response::builder().status(StatusCode::NOT_MODIFIED);
    b = b.header(header::ETAG, super::openapi_etag());
    if let Some(lm) = super::openapi_meta::last_modified_str() {
        b = b.header(header::LAST_MODIFIED, lm);
    }
    b.body(Body::empty()).unwrap()
}

/// Build a 200 OK response with consistent OpenAPI headers.
fn ok_response(body: Body) -> Response {
    let mut b = Response::builder().status(StatusCode::OK);
    b = b.header(header::ETAG, super::openapi_etag());
    b = b.header(header::CONTENT_TYPE, super::OPENAPI_CONTENT_TYPE_JSON);
    b = b.header(header::CONTENT_LENGTH, super::openapi_content_length_str());
    if let Some(lm) = super::openapi_meta::last_modified_str() {
        b = b.header(header::LAST_MODIFIED, lm);
    }
    b.body(body).unwrap()
}

/// GET /openapi.json handler.
///
/// Honors `If-None-Match` (INM) with precedence over `If-Modified-Since` (IMS)
/// per RFC. Returns `304 Not Modified` when appropriate or the full JSON
/// representation with ETag and Content-Length headers.
pub async fn serve_openapi(headers: HeaderMap) -> Response {
    // Centralize the semantics: INM takes precedence over IMS. If INM is
    // present and matches -> 304. If INM is present and does not match ->
    // return the representation (do not consult IMS). If INM is absent,
    // consult IMS.
    if should_return_not_modified(&headers) {
        return not_modified_response();
    }

    let bytes = super::openapi_bytes();
    ok_response(Body::from(bytes.clone()))
}

/// HEAD /openapi.json handler. Same semantics as GET but with an empty body.
pub async fn head_openapi(headers: HeaderMap) -> Response {
    // Same semantics as GET but with an empty body. Use the same precedence
    // logic for INM/IMS to avoid subtle differences between GET and HEAD.
    if should_return_not_modified(&headers) {
        return not_modified_response();
    }

    ok_response(Body::empty())
}

/// Determine whether the request headers indicate the resource is not modified
/// according to RFC rules: If-None-Match takes precedence over
/// If-Modified-Since. Returns `true` when a 304 response should be sent.
fn should_return_not_modified(headers: &HeaderMap) -> bool {
    if headers.contains_key(header::IF_NONE_MATCH) {
        // INM present: only return 304 when there's a match; if present but
        // not matching we must return the representation.
        super::openapi_meta::inm_matches(headers, super::openapi_etag())
    } else {
        // No INM header: safe to consult IMS
        super::openapi_meta::ims_matches(headers)
    }
}
