// src/domain/audit/entity.rs
use crate::domain::user::UserId;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct AuditLog {
    pub id: Option<i64>,
    pub user_id: Option<UserId>,
    pub action: String,
    pub resource_type: String,
    pub resource_id: Option<i64>,
    pub details: Option<serde_json::Value>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
}
