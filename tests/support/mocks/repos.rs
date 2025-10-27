// tests/support/mocks/repos.rs
use async_trait::async_trait;

/* -------------------------------- MockRepo -------------------------------- */

/// 軽量なインメモリ監査ログリポジトリ
/// フィールド経由で戻り値を注入可能
#[derive(Clone, Debug, Default)]
pub struct MockRepo {
    pub items: Vec<mokkan_core::domain::audit::entity::AuditLog>,
    pub next_cursor: Option<String>,
}

impl MockRepo {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_items(items: Vec<mokkan_core::domain::audit::entity::AuditLog>) -> Self {
        Self {
            items,
            next_cursor: None,
        }
    }

    pub fn with(
        items: Vec<mokkan_core::domain::audit::entity::AuditLog>,
        next_cursor: Option<String>,
    ) -> Self {
        Self { items, next_cursor }
    }
}

#[async_trait]
impl mokkan_core::domain::audit::repository::AuditLogRepository for MockRepo {
    async fn insert(
        &self,
        _log: mokkan_core::domain::audit::entity::AuditLog,
    ) -> mokkan_core::domain::errors::DomainResult<()> {
        Ok(())
    }

    async fn list(
        &self,
        _limit: u32,
        _cursor: Option<mokkan_core::domain::audit::cursor::AuditLogCursor>,
    ) -> mokkan_core::domain::errors::DomainResult<(
        Vec<mokkan_core::domain::audit::entity::AuditLog>,
        Option<String>,
    )> {
        Ok((self.items.clone(), self.next_cursor.clone()))
    }

    async fn find_by_user(
        &self,
        _user_id: i64,
        _limit: u32,
        _cursor: Option<mokkan_core::domain::audit::cursor::AuditLogCursor>,
    ) -> mokkan_core::domain::errors::DomainResult<(
        Vec<mokkan_core::domain::audit::entity::AuditLog>,
        Option<String>,
    )> {
        Ok((self.items.clone(), self.next_cursor.clone()))
    }

    async fn find_by_resource(
        &self,
        _resource_type: &str,
        _resource_id: i64,
        _limit: u32,
        _cursor: Option<mokkan_core::domain::audit::cursor::AuditLogCursor>,
    ) -> mokkan_core::domain::errors::DomainResult<(
        Vec<mokkan_core::domain::audit::entity::AuditLog>,
        Option<String>,
    )> {
        Ok((self.items.clone(), self.next_cursor.clone()))
    }
}

/* -------------------------------- MockAuditRepo -------------------------------- */

/// 決定論的な監査ログリポジトリ（一部のE2Eテストで使用）
/// 常に1件のサンプル行を返す
pub struct MockAuditRepo;

#[async_trait]
impl mokkan_core::domain::audit::repository::AuditLogRepository for MockAuditRepo {
    async fn insert(
        &self,
        _log: mokkan_core::domain::audit::entity::AuditLog,
    ) -> mokkan_core::domain::errors::DomainResult<()> {
        Ok(())
    }

    async fn list(
        &self,
        _limit: u32,
        _cursor: Option<mokkan_core::domain::audit::cursor::AuditLogCursor>,
    ) -> mokkan_core::domain::errors::DomainResult<(
        Vec<mokkan_core::domain::audit::entity::AuditLog>,
        Option<String>,
    )> {
        let created_at = super::time::fixed_now();
        Ok((vec![super::audit::sample_audit(created_at)], None))
    }

    async fn find_by_user(
        &self,
        _user_id: i64,
        _limit: u32,
        _cursor: Option<mokkan_core::domain::audit::cursor::AuditLogCursor>,
    ) -> mokkan_core::domain::errors::DomainResult<(
        Vec<mokkan_core::domain::audit::entity::AuditLog>,
        Option<String>,
    )> {
        self.list(_limit, _cursor).await
    }

    async fn find_by_resource(
        &self,
        _resource_type: &str,
        _resource_id: i64,
        _limit: u32,
        _cursor: Option<mokkan_core::domain::audit::cursor::AuditLogCursor>,
    ) -> mokkan_core::domain::errors::DomainResult<(
        Vec<mokkan_core::domain::audit::entity::AuditLog>,
        Option<String>,
    )> {
        self.list(_limit, _cursor).await
    }
}

/* -------------------------------- CapturingAuditRepo -------------------------------- */

/// 挿入された値をキャプチャする監査ログリポジトリ
/// テストでinsertが特定の値で呼ばれたことをアサートする際に有用
#[derive(Clone, Default)]
pub struct CapturingAuditRepo {
    pub items: Vec<mokkan_core::domain::audit::entity::AuditLog>,
    pub next_cursor: Option<String>,
    pub inserted:
        std::sync::Arc<std::sync::Mutex<Vec<mokkan_core::domain::audit::entity::AuditLog>>>,
}

impl CapturingAuditRepo {
    pub fn new() -> Self {
        Self {
            items: vec![],
            next_cursor: None,
            inserted: std::sync::Arc::new(std::sync::Mutex::new(vec![])),
        }
    }

    /// 挿入された全てのログを取得
    pub fn get_inserted(&self) -> Vec<mokkan_core::domain::audit::entity::AuditLog> {
        self.inserted.lock().expect("mutex poisoned").clone()
    }
}

#[async_trait]
impl mokkan_core::domain::audit::repository::AuditLogRepository for CapturingAuditRepo {
    async fn insert(
        &self,
        log: mokkan_core::domain::audit::entity::AuditLog,
    ) -> mokkan_core::domain::errors::DomainResult<()> {
        let mut guard = self.inserted.lock().expect("mutex poisoned");
        guard.push(log);
        Ok(())
    }

    async fn list(
        &self,
        _limit: u32,
        _cursor: Option<mokkan_core::domain::audit::cursor::AuditLogCursor>,
    ) -> mokkan_core::domain::errors::DomainResult<(
        Vec<mokkan_core::domain::audit::entity::AuditLog>,
        Option<String>,
    )> {
        Ok((self.items.clone(), self.next_cursor.clone()))
    }

    async fn find_by_user(
        &self,
        _user_id: i64,
        _limit: u32,
        _cursor: Option<mokkan_core::domain::audit::cursor::AuditLogCursor>,
    ) -> mokkan_core::domain::errors::DomainResult<(
        Vec<mokkan_core::domain::audit::entity::AuditLog>,
        Option<String>,
    )> {
        Ok((self.items.clone(), self.next_cursor.clone()))
    }

    async fn find_by_resource(
        &self,
        _resource_type: &str,
        _resource_id: i64,
        _limit: u32,
        _cursor: Option<mokkan_core::domain::audit::cursor::AuditLogCursor>,
    ) -> mokkan_core::domain::errors::DomainResult<(
        Vec<mokkan_core::domain::audit::entity::AuditLog>,
        Option<String>,
    )> {
        Ok((self.items.clone(), self.next_cursor.clone()))
    }
}
