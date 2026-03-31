// tests/support/mocks/repos.rs
use mokkan_core::async_support::{BoxFuture, boxed};

/* -------------------------------- MockRepo -------------------------------- */

/// 軽量なインメモリ監査ログリポジトリ
/// フィールド経由で戻り値を注入可能
#[derive(Clone, Debug, Default)]
#[must_use]
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

impl mokkan_core::domain::audit::repository::AuditLogRepository for MockRepo {
    fn insert(
        &self,
        _log: mokkan_core::domain::audit::entity::NewAuditLog,
    ) -> BoxFuture<'_, mokkan_core::domain::errors::DomainResult<()>> {
        boxed(async move { Ok(()) })
    }

    fn list(
        &self,
        _limit: u32,
        _cursor: Option<mokkan_core::domain::audit::cursor::Cursor>,
    ) -> BoxFuture<
        '_,
        mokkan_core::domain::errors::DomainResult<(
            Vec<mokkan_core::domain::audit::entity::AuditLog>,
            Option<String>,
        )>,
    > {
        boxed(async move { Ok((self.items.clone(), self.next_cursor.clone())) })
    }

    fn find_by_user(
        &self,
        _user_id: i64,
        _limit: u32,
        _cursor: Option<mokkan_core::domain::audit::cursor::Cursor>,
    ) -> BoxFuture<
        '_,
        mokkan_core::domain::errors::DomainResult<(
            Vec<mokkan_core::domain::audit::entity::AuditLog>,
            Option<String>,
        )>,
    > {
        boxed(async move { Ok((self.items.clone(), self.next_cursor.clone())) })
    }

    fn find_by_resource<'a>(
        &'a self,
        _resource_type: &str,
        _resource_id: i64,
        _limit: u32,
        _cursor: Option<mokkan_core::domain::audit::cursor::Cursor>,
    ) -> BoxFuture<
        'a,
        mokkan_core::domain::errors::DomainResult<(
            Vec<mokkan_core::domain::audit::entity::AuditLog>,
            Option<String>,
        )>,
    > {
        boxed(async move { Ok((self.items.clone(), self.next_cursor.clone())) })
    }
}

/* -------------------------------- MockAuditRepo -------------------------------- */

/// 決定論的な監査ログリポジトリ（一部のE2Eテストで使用）
/// 常に1件のサンプル行を返す
pub struct MockAuditRepo;

impl mokkan_core::domain::audit::repository::AuditLogRepository for MockAuditRepo {
    fn insert(
        &self,
        _log: mokkan_core::domain::audit::entity::NewAuditLog,
    ) -> BoxFuture<'_, mokkan_core::domain::errors::DomainResult<()>> {
        boxed(async move { Ok(()) })
    }

    fn list(
        &self,
        _limit: u32,
        _cursor: Option<mokkan_core::domain::audit::cursor::Cursor>,
    ) -> BoxFuture<
        '_,
        mokkan_core::domain::errors::DomainResult<(
            Vec<mokkan_core::domain::audit::entity::AuditLog>,
            Option<String>,
        )>,
    > {
        boxed(async move {
            let created_at = super::time::fixed_now();
            Ok((vec![super::audit::sample(created_at)], None))
        })
    }

    fn find_by_user(
        &self,
        _user_id: i64,
        _limit: u32,
        _cursor: Option<mokkan_core::domain::audit::cursor::Cursor>,
    ) -> BoxFuture<
        '_,
        mokkan_core::domain::errors::DomainResult<(
            Vec<mokkan_core::domain::audit::entity::AuditLog>,
            Option<String>,
        )>,
    > {
        boxed(async move { self.list(_limit, _cursor).await })
    }

    fn find_by_resource<'a>(
        &'a self,
        _resource_type: &str,
        _resource_id: i64,
        _limit: u32,
        _cursor: Option<mokkan_core::domain::audit::cursor::Cursor>,
    ) -> BoxFuture<
        'a,
        mokkan_core::domain::errors::DomainResult<(
            Vec<mokkan_core::domain::audit::entity::AuditLog>,
            Option<String>,
        )>,
    > {
        boxed(async move { self.list(_limit, _cursor).await })
    }
}

/* -------------------------------- CapturingAuditRepo -------------------------------- */

/// 挿入された値をキャプチャする監査ログリポジトリ
/// テストでinsertが特定の値で呼ばれたことをアサートする際に有用
#[derive(Clone, Default)]
#[must_use]
pub struct CapturingAuditRepo {
    pub items: Vec<mokkan_core::domain::audit::entity::AuditLog>,
    pub next_cursor: Option<String>,
    pub inserted:
        std::sync::Arc<std::sync::Mutex<Vec<mokkan_core::domain::audit::entity::NewAuditLog>>>,
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
    pub fn get_inserted(&self) -> Vec<mokkan_core::domain::audit::entity::NewAuditLog> {
        self.inserted.lock().expect("mutex poisoned").clone()
    }
}

impl mokkan_core::domain::audit::repository::AuditLogRepository for CapturingAuditRepo {
    fn insert(
        &self,
        log: mokkan_core::domain::audit::entity::NewAuditLog,
    ) -> BoxFuture<'_, mokkan_core::domain::errors::DomainResult<()>> {
        boxed(async move {
            let mut guard = self.inserted.lock().expect("mutex poisoned");
            guard.push(log);
            drop(guard);
            Ok(())
        })
    }

    fn list(
        &self,
        _limit: u32,
        _cursor: Option<mokkan_core::domain::audit::cursor::Cursor>,
    ) -> BoxFuture<
        '_,
        mokkan_core::domain::errors::DomainResult<(
            Vec<mokkan_core::domain::audit::entity::AuditLog>,
            Option<String>,
        )>,
    > {
        boxed(async move { Ok((self.items.clone(), self.next_cursor.clone())) })
    }

    fn find_by_user(
        &self,
        _user_id: i64,
        _limit: u32,
        _cursor: Option<mokkan_core::domain::audit::cursor::Cursor>,
    ) -> BoxFuture<
        '_,
        mokkan_core::domain::errors::DomainResult<(
            Vec<mokkan_core::domain::audit::entity::AuditLog>,
            Option<String>,
        )>,
    > {
        boxed(async move { Ok((self.items.clone(), self.next_cursor.clone())) })
    }

    fn find_by_resource<'a>(
        &'a self,
        _resource_type: &str,
        _resource_id: i64,
        _limit: u32,
        _cursor: Option<mokkan_core::domain::audit::cursor::Cursor>,
    ) -> BoxFuture<
        'a,
        mokkan_core::domain::errors::DomainResult<(
            Vec<mokkan_core::domain::audit::entity::AuditLog>,
            Option<String>,
        )>,
    > {
        boxed(async move { Ok((self.items.clone(), self.next_cursor.clone())) })
    }
}
