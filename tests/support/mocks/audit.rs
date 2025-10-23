// tests/support/mocks/audit.rs
use chrono::{DateTime, Utc};

pub fn sample_audit(created_at: DateTime<Utc>) -> mokkan_core::domain::audit::entity::AuditLog {
    mokkan_core::domain::audit::entity::AuditLog {
        id: Some(1),
        user_id: Some(mokkan_core::domain::user::value_objects::UserId::new(1).unwrap()),
        action: "test".into(),
        resource_type: "article".into(),
        resource_id: Some(100),
        details: None,
        ip_address: Some("127.0.0.1".into()),
        user_agent: Some("e2e-test".into()),
        created_at: Some(created_at),
    }
}

pub fn sample_audit_with(id: i64, resource_id: i64, created_at: DateTime<Utc>) -> mokkan_core::domain::audit::entity::AuditLog {
    let mut a = sample_audit(created_at);
    a.id = Some(id);
    a.resource_id = Some(resource_id);
    a
}
