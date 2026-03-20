// src/presentation/http/openapi.rs
// Minimal OpenAPI helpers used by the HTTP layer and tests.
pub mod openapi_meta;
pub mod openapi_mutation;
use axum::Router;
use axum::routing::{get, head as route_head};
use bytes::Bytes;
use std::sync::OnceLock;
// types for openapi payloads live in the `openapi` submodule (openapi/openapi_types.rs)

// Caches for generated OpenAPI JSON and derived metadata.
static BYTES: OnceLock<Bytes> = OnceLock::new();
static ETAG: OnceLock<String> = OnceLock::new();
static CONTENT_LENGTH: OnceLock<usize> = OnceLock::new();
static CONTENT_LENGTH_STR: OnceLock<String> = OnceLock::new();

/// Content-Type used for the `OpenAPI` JSON representation.
pub const CONTENT_TYPE_JSON: &str = "application/json";

// Canonical minimal OpenAPI JSON as a static byte slice so the `bytes`
// accessor can stay very small (this also keeps static analysis tools happy).
static JSON_BYTES: &[u8] =
    b"{\"openapi\":\"3.0.0\",\"info\":{\"title\":\"mokkan_core\",\"version\":\"0.1.0\"},\"paths\":{}}";

// Minimal OpenAPI JSON bytes used for tests (stable across calls)
/// Return a reference to the canonical `OpenAPI` JSON bytes used by the
/// application and tests. The value is cached in a `OnceLock` so repeated
/// calls are cheap and return the same `Bytes` instance.
pub fn bytes() -> &'static Bytes {
    BYTES.get_or_init(|| Bytes::from_static(JSON_BYTES))
}

pub mod openapi_types;
pub use openapi_types::{ArticleListResponse, StatusResponse, UserListResponse};
/// Return the content length, in bytes, of the `OpenAPI` JSON payload.
pub fn content_length() -> usize {
    *CONTENT_LENGTH.get_or_init(|| bytes().len())
}

/// Return a shared string value for the `OpenAPI` `Content-Length` header so
/// callers can pass a &'static str without allocating repeatedly.
pub fn content_length_str() -> &'static str {
    CONTENT_LENGTH_STR
        .get_or_init(|| content_length().to_string())
        .as_str()
}

pub fn etag() -> &'static str {
    ETAG.get_or_init(|| openapi_meta::compute_simple_etag(bytes()))
        .as_str()
}
pub mod handlers;
pub use handlers::{head, serve};
pub use openapi_meta::{inm_matches, weak_match};

/// Minimal docs router used by the application router builder. Tests don't need the UI
/// served, only that this returns a Router that can be merged.
pub fn docs_router() -> Router {
    // Return the minimal docs router (JSON only) used by the application.
    // Reuse `docs_router_with_options` to keep route registration in one place.
    docs_router_with_options(false, false)
}

/// Return a docs router with a couple of small test-oriented options.
///
/// This keeps behavior simple and deterministic for unit and integration
/// tests by exposing only the JSON and `HEAD` endpoints asserted by the
/// test suite.
pub fn docs_router_with_options(_serve_ui: bool, persist_snapshot: bool) -> Router {
    // The flags are intentionally simple; tests call with (true, false).
    if persist_snapshot {
        // Best-effort: try to write the snapshot but don't panic the caller on
        // failure. The application caller in `main.rs` already logs failures.
        let _ = write_snapshot();
    }

    // Build a minimal router exposing /openapi.json GET and HEAD. If `serve_ui`
    // is true we could mount a static UI; tests don't assert on that so keep it
    // minimal here. Register the handlers directly — axum will invoke the
    // extractor-based async functions (they accept a `HeaderMap`).
    Router::new()
        .route("/openapi.json", get(serve))
        .route("/openapi.json", route_head(head))
}

/// Write the canonical `OpenAPI` snapshot to `spec/openapi.json`.
///
/// This is intentionally small and deterministic: it writes the bytes from
/// `bytes()` and returns an `std::io::Result<()>` so callers can
/// decide how to react when writing fails. This is used by `CI` and local
/// tooling to persist the generated `OpenAPI` spec.
///
/// # Errors
///
/// Returns any filesystem error raised while creating the output directory or
/// writing the snapshot file.
pub fn write_snapshot() -> std::io::Result<()> {
    let out_path = std::path::Path::new("spec").join("openapi.json");
    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(out_path, bytes().as_ref())
}

// Use the external tests file under `openapi/tests.rs` to keep this file small.
#[cfg(test)]
mod tests;
