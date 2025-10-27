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
    // If If-None-Match header is present, it takes precedence over
    // If-Modified-Since (RFC): if INM matches -> 304, if INM is present but
    // does not match -> return the full representation (do not consult IMS).
    if headers.contains_key(header::IF_NONE_MATCH) {
        if super::openapi_meta::inm_matches(&headers, super::openapi_etag()) {
            return not_modified_response();
        }
        // INM present and didn't match: fall-through to return representation
    } else {
        // No INM header: safe to consult If-Modified-Since
        if super::openapi_meta::ims_matches(&headers) {
            return not_modified_response();
        }
    }

    let bytes = super::openapi_bytes();
    ok_response(Body::from(bytes.clone()))
}

/// HEAD /openapi.json handler. Same semantics as GET but with an empty body.
pub async fn head_openapi(headers: HeaderMap) -> Response {
    // similar to GET but with no body
    if super::openapi_meta::inm_matches(&headers, super::openapi_etag()) {
        return not_modified_response();
    }

    if super::openapi_meta::ims_matches(&headers) {
        return not_modified_response();
    }

    ok_response(Body::empty())
}
