#[tokio::test]
async fn integration_audit_write_and_read() {
    // Run only when explicitly enabled to avoid requiring Postgres in all environments
    if std::env::var("RUN_DB_INTEGRATION").unwrap_or_default() != "1" {
        eprintln!("skipping integration test: set RUN_DB_INTEGRATION=1 and DATABASE_URL to run");
        return;
    }

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for integration tests");
    let pool = mokkan_core::infrastructure::database::init_pool(&database_url)
        .await
        .expect("init pool");
    // apply migrations to ensure schema exists
    mokkan_core::infrastructure::database::run_migrations(&pool)
        .await
        .expect("run migrations");

    use std::sync::Arc;
    use mokkan_core::domain::audit::repository::AuditLogRepository;

    let repo_impl = mokkan_core::infrastructure::repositories::PostgresAuditLogRepository::new(pool.clone());
    let repo: Arc<dyn AuditLogRepository> = Arc::new(repo_impl);

    // insert test rows
    for i in 0..5i64 {
        let log = mokkan_core::domain::audit::entity::AuditLog {
            id: None,
            user_id: Some(mokkan_core::domain::user::value_objects::UserId::new(1).unwrap()),
            action: format!("test-integration-{}", i),
            resource_type: "article".to_string(),
            resource_id: Some(100 + i),
            details: Some(serde_json::json!({"i": i})),
            ip_address: Some("127.0.0.1".to_string()),
            user_agent: Some("mokkan-integration-test".to_string()),
            created_at: None,
        };
        repo.insert(log).await.expect("insert");
    }

    // query with small limit and expect a next_cursor
    let (items, next_cursor) = repo.list(2, None).await.expect("list");
    assert!(items.len() >= 2, "expected at least 2 items");
    assert!(next_cursor.is_some(), "expected next_cursor when more items exist");

    // cleanup test rows
    sqlx::query("DELETE FROM audit_logs WHERE action LIKE 'test-integration-%'")
        .execute(&pool)
        .await
        .expect("cleanup");
}
