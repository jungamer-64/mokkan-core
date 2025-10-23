// src/presentation/http/middleware/audit.rs
use crate::domain::audit::entity::AuditLog;
use crate::presentation::http::extractors::MaybeAuthenticated;
use crate::presentation::http::state::HttpState;
use axum::{body::Body, extract::Extension, http, middleware::Next, response::Response, Request};
use tracing::warn;

pub async fn audit_middleware(
    MaybeAuthenticated(user): MaybeAuthenticated,
    Extension(state): Extension<HttpState>,
    mut req: Request<Body>,
    next: Next<Body>,
) -> Response {
    let method = req.method().clone();
    let uri = req.uri().path().to_string();

    let headers = req.headers().clone();

    let response = next.run(req).await;
    let status = response.status().as_u16() as i32;

    // only log write-like operations
    if matches!(method, &http::Method::POST | &http::Method::PUT | &http::Method::PATCH | &http::Method::DELETE) {
        // get repo from services and spawn background task
        let repo = state.services.audit_log_repo();
        let user_id = user.map(|u| u.0.id);
        let action = format!("{} {} -> {}", method, uri, status);
        let resource_type = "http_request".to_string();
        let resource_id = None;
        let details = None;
        let ip_address = headers
            .get("x-forwarded-for")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());
        let user_agent = headers
            .get(http::header::USER_AGENT)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        tokio::spawn(async move {
            let log = AuditLog {
                user_id: user_id.map(Into::into),
                action,
                resource_type,
                resource_id,
                details,
                ip_address,
                user_agent,
            };

            if let Err(e) = repo.insert(log).await {
                warn!(error = %e, "failed to insert audit log");
            }
        });
    }

    response
}
