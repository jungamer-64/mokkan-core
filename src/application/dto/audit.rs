use crate::domain::audit::entity::AuditLog;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AuditLogDto {
    pub id: i64,
    pub user_id: Option<i64>,
    pub action: String,
    pub resource_type: String,
    pub resource_id: Option<i64>,
    pub details: Option<serde_json::Value>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
}

impl From<AuditLog> for AuditLogDto {
    fn from(a: AuditLog) -> Self {
        Self {
            id: a.id.unwrap_or_default(),
            user_id: a.user_id.map(Into::into),
            action: a.action,
            resource_type: a.resource_type,
            resource_id: a.resource_id,
            details: a.details,
            ip_address: a.ip_address,
            user_agent: a.user_agent,
        }
    }
}
