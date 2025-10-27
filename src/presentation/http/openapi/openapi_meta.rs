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
