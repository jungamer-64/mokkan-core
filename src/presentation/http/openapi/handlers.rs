use axum::{
    body::Body,
    http::{HeaderMap, StatusCode, header},
    response::Response,
};

/// Small helper: strip optional weak prefix (`W/` or `w/`) from a token.
///
/// Does not alter surrounding quotes — it only removes the weak prefix when
/// present to simplify downstream normalization.
fn strip_weak_prefix_str(s: &str) -> &str {
    if s.len() > 2 && (s.starts_with("W/") || s.starts_with("w/")) {
        &s[2..]
    } else {
        s
    }
}

/// Unescape simple backslash escapes ("\\" + any char -> that char).
///
/// This handles common ETag encodings like `\"` -> `"` used in some
/// header representations.
fn unescape_simple(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(nc) = chars.next() {
                out.push(nc);
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// Extract the ETag opaque value from a token.
///
/// Strips an optional weak prefix (`W/`), removes surrounding quotes, and
/// unescapes simple backslash escapes so that semantically-equal ETags such
/// as `W/"bar"` and `"bar"` normalize to the same value.
fn extract_etag_value(token: &str) -> String {
    let t = strip_weak_prefix_str(token).trim();

    // if the token is quoted, strip the surrounding quotes before
    // unescaping; otherwise unescape directly
    let inner = if t.len() >= 2 && t.starts_with('"') && t.ends_with('"') {
        &t[1..t.len() - 1]
    } else {
        t
    };

    let mut out = unescape_simple(inner);

    // If after unescaping we still have surrounding quotes (e.g. token was
    // written with explicit backslash-escaped quotes in source like
    // `W/\"bar\"`), strip them so both representations normalize to the
    // same opaque value.
    if out.len() >= 2 && out.starts_with('"') && out.ends_with('"') {
        out = out[1..out.len() - 1].to_string();
    }

    out
}

/// Compare two ETag tokens for weak-equivalence. Both inputs are normalized
/// via `extract_etag_value` before comparison so different textual forms of
/// the same opaque value compare equal.
pub fn weak_match(a: &str, b: &str) -> bool {
    extract_etag_value(a) == extract_etag_value(b)
}

/// Check whether the request `If-None-Match` header matches the `actual`
/// ETag value. Supports the `*` wildcard and comma-separated candidate
/// lists. Returns `true` if any candidate weakly matches `actual`.
pub fn inm_matches(headers: &HeaderMap, actual: &str) -> bool {
    if let Some(v) = headers.get(header::IF_NONE_MATCH)
        && let Ok(sv) = v.to_str()
    {
        let s = sv.trim();
        if s == "*" {
            return true;
        }
        for candidate in s.split(',').map(str::trim) {
            if weak_match(candidate, actual) {
                return true;
            }
        }
    }
    false
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
        if inm_matches(&headers, super::openapi_etag()) {
            return Response::builder()
                .status(StatusCode::NOT_MODIFIED)
                .header(header::ETAG, super::openapi_etag())
                .body(Body::empty())
                .unwrap();
        }
        // INM present and didn't match: fall-through to return representation
    } else {
        // No INM header: safe to consult If-Modified-Since
        if let Some(v) = headers.get(header::IF_MODIFIED_SINCE)
            && let Ok(s) = v.to_str()
            && let Some(lm) = super::openapi_meta::last_modified_str()
            && s == lm
        {
            return Response::builder()
                .status(StatusCode::NOT_MODIFIED)
                .header(header::ETAG, super::openapi_etag())
                .body(Body::empty())
                .unwrap();
        }
    }

    let bytes = super::openapi_bytes();
    Response::builder()
        .status(StatusCode::OK)
        .header(header::ETAG, super::openapi_etag())
        .header(header::CONTENT_TYPE, "application/json")
        .header(
            header::CONTENT_LENGTH,
            super::openapi_content_length().to_string(),
        )
        .body(Body::from(bytes.clone()))
        .unwrap()
}

/// HEAD /openapi.json handler. Same semantics as GET but with an empty body.
pub async fn head_openapi(headers: HeaderMap) -> Response {
    // similar to GET but with no body
    if inm_matches(&headers, super::openapi_etag()) {
        return Response::builder()
            .status(StatusCode::NOT_MODIFIED)
            .header(header::ETAG, super::openapi_etag())
            .body(Body::empty())
            .unwrap();
    }

    if let Some(v) = headers.get(header::IF_MODIFIED_SINCE)
        && let Ok(s) = v.to_str()
        && let Some(lm) = super::openapi_meta::last_modified_str()
        && s == lm
    {
        return Response::builder()
            .status(StatusCode::NOT_MODIFIED)
            .header(header::ETAG, super::openapi_etag())
            .body(Body::empty())
            .unwrap();
    }

    Response::builder()
        .status(StatusCode::OK)
        .header(header::ETAG, super::openapi_etag())
        .header(header::CONTENT_TYPE, "application/json")
        .header(
            header::CONTENT_LENGTH,
            super::openapi_content_length().to_string(),
        )
        .body(Body::empty())
        .unwrap()
}
