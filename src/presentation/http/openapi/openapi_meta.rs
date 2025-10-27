use axum::http::{HeaderMap, header};
use bytes::Bytes;
use std::borrow::Cow;
use std::sync::OnceLock;
use std::time::SystemTime;

// process-startup timestamp for Last-Modified when BUILD_DATE is not set
pub static STARTUP_DATE: OnceLock<String> = OnceLock::new();

/// Return the Last-Modified string the server actually sends (BUILD_DATE or STARTUP_DATE)
pub fn last_modified_str() -> Option<&'static str> {
    if let Some(v) = option_env!("BUILD_DATE") {
        Some(v)
    } else {
        Some(
            STARTUP_DATE
                .get_or_init(|| httpdate::fmt_http_date(SystemTime::now()))
                .as_str(),
        )
    }
}

/// Compute a small deterministic ETag for the given bytes.
///
/// This uses a simple 64-bit FNV-1a style rolling hash. It's intentionally
/// minimal to avoid adding dependencies; the resulting hex value is quoted
/// to be a valid ETag token (e.g. `"abc123"`).
pub(crate) fn compute_simple_etag(b: &Bytes) -> String {
    // FNV-1a 64-bit offset basis (see https://en.wikipedia.org/wiki/Fowler–Noll–Vo_hash_function)
    let mut h: u64 = 1469598103934665603u64;
    for &byte in b.iter() {
        // FNV-1a 64-bit prime (see https://en.wikipedia.org/wiki/Fowler–Noll–Vo_hash_function)
        h = h.wrapping_mul(1099511628211u64) ^ (byte as u64);
    }
    format!("\"{:x}\"", h)
}

/// Small helper: strip optional weak prefix (`W/` or `w/`) from a token.
///
/// Does not alter surrounding quotes — it only removes the weak prefix when
/// present to simplify downstream normalization.
pub(crate) fn strip_weak_prefix_str(s: &str) -> &str {
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
pub fn unescape_simple(s: &str) -> String {
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
/// as `W/\"bar\"` and `\"bar\"` normalize to the same value.
pub fn extract_etag_value(token: &str) -> Cow<'_, str> {
    let t = strip_weak_prefix_str(token).trim();

    // if the token is quoted, strip the surrounding quotes before
    // unescaping; otherwise unescape directly
    let inner = if t.len() >= 2 && t.starts_with('"') && t.ends_with('"') {
        &t[1..t.len() - 1]
    } else {
        t
    };

    // Fast path: if there are no backslash escapes we can borrow the slice
    // directly which avoids allocating a new String in the common case.
    if !inner.contains('\\') {
        // If the inner slice itself still has surrounding quotes (possible
        // when the value was written with escaped quotes), return an owned
        // stripped string; otherwise borrow.
        if inner.len() >= 2 && inner.starts_with('"') && inner.ends_with('"') {
            return Cow::Owned(inner[1..inner.len() - 1].to_string());
        }
        return Cow::Borrowed(inner);
    }

    // Slow path: unescape into a new String and strip any remaining quotes.
    let mut out = unescape_simple(inner);
    if out.len() >= 2 && out.starts_with('"') && out.ends_with('"') {
        out = out[1..out.len() - 1].to_string();
    }
    Cow::Owned(out)
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
    if let Some(v) = headers.get(header::IF_NONE_MATCH) {
        if let Ok(sv) = v.to_str() {
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
    }
    false
}

/// Check whether the request `If-Modified-Since` header matches the
/// server's Last-Modified value. Returns `true` when present and equal.
pub fn ims_matches(headers: &HeaderMap) -> bool {
    if let Some(v) = headers.get(header::IF_MODIFIED_SINCE) {
        if let Ok(s) = v.to_str() {
            if let Some(lm) = last_modified_str() {
                return s == lm;
            }
        }
    }
    false
}
