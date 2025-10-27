// tests/audit_query_tests.rs
use mokkan_core::application::dto::AuthenticatedUser;
use mokkan_core::application::queries::audit::{AuditQueryService, ListAuditLogsQuery};
use mokkan_core::domain::user::value_objects::Capability;
use mokkan_core::domain::user::value_objects::UserId;
// domain errors are not needed in this test
use std::sync::Arc;
mod support;
use support::MockRepo;

#[tokio::test]
async fn audit_query_service_list_decodes_cursor_and_returns_page() {
    let repo = MockRepo {
        items: vec![],
        next_cursor: None,
    };
    let svc = AuditQueryService::new(Arc::new(repo));

    let auth = AuthenticatedUser {
        id: UserId::new(1).unwrap(),
        username: "tester".into(),
        role: mokkan_core::domain::user::value_objects::Role::Admin,
        capabilities: std::collections::HashSet::from([Capability::new("audit", "read")]),
        issued_at: chrono::Utc::now(),
        expires_at: chrono::Utc::now(),
        session_id: None,
        token_version: None,
    };

    let q = ListAuditLogsQuery {
        limit: 10,
        cursor: None,
    };
    let res = svc.list_audit_logs(&auth, q).await;
    assert!(res.is_ok());
    let page = res.unwrap();
    assert_eq!(page.items.len(), 0);
}
