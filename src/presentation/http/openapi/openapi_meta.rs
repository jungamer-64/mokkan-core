use bytes::Bytes;
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
    let mut h: u64 = 1469598103934665603u64;
    for &byte in b.iter() {
        h = h.wrapping_mul(1099511628211u64) ^ (byte as u64);
    }
    format!("\"{:x}\"", h)
}
