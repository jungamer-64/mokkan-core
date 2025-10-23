// tests/support/mocks.rs
//! Minimal, canonical mocks shared by unit/E2E tests. Keep this file small and stable.

use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use std::collections::HashSet;

fn fixed_now() -> DateTime<Utc> {
    // Deterministic timestamp for tests
    DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z")
        .unwrap()
        .with_timezone(&Utc)
}

/* ------------------------------ Security ------------------------------ */

pub struct DummyTokenManager;

#[async_trait]
impl mokkan_core::application::ports::security::TokenManager for DummyTokenManager {
    async fn issue(
        &self,
        _subject: mokkan_core::application::dto::TokenSubject,
    ) -> mokkan_core::application::ApplicationResult<
        mokkan_core::application::dto::AuthTokenDto
    > {
        Err(mokkan_core::application::error::ApplicationError::infrastructure("not implemented"))
    }

    async fn authenticate(
        &self,
        token: &str,
    ) -> mokkan_core::application::ApplicationResult<
        mokkan_core::application::dto::AuthenticatedUser
    > {
        let now = fixed_now();
        match token {
            // Admin + audit:read, valid 1h
            "test-token" => Ok(mokkan_core::application::dto::AuthenticatedUser {
                id: mokkan_core::domain::user::value_objects::UserId::new(1).unwrap(),
                username: "tester".into(),
                role: mokkan_core::domain::user::value_objects::Role::Admin,
                capabilities: HashSet::from([
                    mokkan_core::domain::user::value_objects::Capability::new("audit", "read"),
                ]),
                issued_at: now,
                expires_at: now + Duration::hours(1),
            }),
            // Author, no capabilities
            "no-audit" => Ok(mokkan_core::application::dto::AuthenticatedUser {
                id: mokkan_core::domain::user::value_objects::UserId::new(2).unwrap(),
                username: "noaudit".into(),
                role: mokkan_core::domain::user::value_objects::Role::Author,
                capabilities: HashSet::new(),
                issued_at: now,
                expires_at: now + Duration::hours(1),
            }),
            // Admin + audit:read, but already expired
            "expired-token" => Ok(mokkan_core::application::dto::AuthenticatedUser {
                id: mokkan_core::domain::user::value_objects::UserId::new(3).unwrap(),
                username: "expired".into(),
                role: mokkan_core::domain::user::value_objects::Role::Admin,
                capabilities: HashSet::from([
                    mokkan_core::domain::user::value_objects::Capability::new("audit", "read"),
                ]),
                issued_at: now - Duration::hours(2),
                expires_at: now - Duration::hours(1),
            }),
            _ => Err(mokkan_core::application::error::ApplicationError::unauthorized("invalid token")),
        }
    }
}

pub struct DummyPasswordHasher;

#[async_trait]
impl mokkan_core::application::ports::security::PasswordHasher for DummyPasswordHasher {
    async fn hash(&self, _password: &str) -> mokkan_core::application::ApplicationResult<String> {
        Ok("hash".into())
    }
    async fn verify(&self, _password: &str, _expected_hash: &str) -> mokkan_core::application::ApplicationResult<()> {
        Ok(())
    }
}

/* --------------------------------- Time -------------------------------- */

pub struct DummyClock;

impl mokkan_core::application::ports::time::Clock for DummyClock {
    fn now(&self) -> DateTime<Utc> {
        // Use fixed time for deterministic tests
        fixed_now()
    }
}

/* -------------------------------- Utils -------------------------------- */

pub struct DummySlug;

impl mokkan_core::application::ports::util::SlugGenerator for DummySlug {
    fn slugify(&self, s: &str) -> String {
        s.to_string()
    }
}

/* ------------------------------- Audit repo ---------------------------- */

/// Lightweight in-memory repo whose return values can be injected via fields.
pub struct MockRepo {
    pub items: Vec<mokkan_core::domain::audit::entity::AuditLog>,
    pub next_cursor: Option<String>,
}

#[async_trait]
impl mokkan_core::domain::audit::repository::AuditLogRepository for MockRepo {
    async fn insert(&self, _log: mokkan_core::domain::audit::entity::AuditLog) -> mokkan_core::domain::errors::DomainResult<()> {
        Ok(())
    }
    async fn list(
        &self,
        _limit: u32,
        _cursor: Option<mokkan_core::domain::audit::cursor::AuditLogCursor>,
    ) -> mokkan_core::domain::errors::DomainResult<(Vec<mokkan_core::domain::audit::entity::AuditLog>, Option<String>)> {
        Ok((self.items.clone(), self.next_cursor.clone()))
    }
    async fn find_by_user(
        &self,
        _user_id: i64,
        _limit: u32,
        _cursor: Option<mokkan_core::domain::audit::cursor::AuditLogCursor>,
    ) -> mokkan_core::domain::errors::DomainResult<(Vec<mokkan_core::domain::audit::entity::AuditLog>, Option<String>)> {
        Ok((self.items.clone(), self.next_cursor.clone()))
    }
    async fn find_by_resource(
        &self,
        _resource_type: &str,
        _resource_id: i64,
        _limit: u32,
        _cursor: Option<mokkan_core::domain::audit::cursor::AuditLogCursor>,
    ) -> mokkan_core::domain::errors::DomainResult<(Vec<mokkan_core::domain::audit::entity::AuditLog>, Option<String>)> {
        Ok((self.items.clone(), self.next_cursor.clone()))
    }
}

/// Deterministic repo used by some E2E tests. Always returns one sample row.
pub struct MockAuditRepo;

#[async_trait]
impl mokkan_core::domain::audit::repository::AuditLogRepository for MockAuditRepo {
    async fn insert(&self, _log: mokkan_core::domain::audit::entity::AuditLog) -> mokkan_core::domain::errors::DomainResult<()> { Ok(()) }
    async fn list(
        &self,
        _limit: u32,
        _cursor: Option<mokkan_core::domain::audit::cursor::AuditLogCursor>,
    ) -> mokkan_core::domain::errors::DomainResult<(Vec<mokkan_core::domain::audit::entity::AuditLog>, Option<String>)> {
        let created_at = fixed_now();
        let sample = mokkan_core::domain::audit::entity::AuditLog {
            id: Some(1),
            user_id: Some(mokkan_core::domain::user::value_objects::UserId::new(1).unwrap()),
            action: "test".into(),
            resource_type: "article".into(),
            resource_id: Some(100),
            details: None,
            ip_address: Some("127.0.0.1".into()),
            user_agent: Some("e2e-test".into()),
            created_at: Some(created_at),
        };
        Ok((vec![sample], None))
    }
    async fn find_by_user(
        &self,
        _user_id: i64,
        _limit: u32,
        _cursor: Option<mokkan_core::domain::audit::cursor::AuditLogCursor>,
    ) -> mokkan_core::domain::errors::DomainResult<(Vec<mokkan_core::domain::audit::entity::AuditLog>, Option<String>)> {
        self.list(_limit, _cursor).await
    }
    async fn find_by_resource(
        &self,
        _resource_type: &str,
        _resource_id: i64,
        _limit: u32,
        _cursor: Option<mokkan_core::domain::audit::cursor::AuditLogCursor>,
    ) -> mokkan_core::domain::errors::DomainResult<(Vec<mokkan_core::domain::audit::entity::AuditLog>, Option<String>)> {
        self.list(_limit, _cursor).await
    }
}

/* ------------------------------- User repo ----------------------------- */

pub struct DummyUserRepo;

#[async_trait]
impl mokkan_core::domain::user::repository::UserRepository for DummyUserRepo {
    async fn count(&self) -> mokkan_core::domain::errors::DomainResult<u64> { Ok(0) }
    async fn insert(
        &self,
        _new_user: mokkan_core::domain::user::entity::NewUser,
    ) -> mokkan_core::domain::errors::DomainResult<mokkan_core::domain::user::entity::User> {
        Err(mokkan_core::domain::errors::DomainError::NotFound("not implemented".into()))
    }
    async fn find_by_username(
        &self,
        _username: &mokkan_core::domain::user::value_objects::Username,
    ) -> mokkan_core::domain::errors::DomainResult<Option<mokkan_core::domain::user::entity::User>> {
        Ok(None)
    }
    async fn find_by_id(
        &self,
        _id: mokkan_core::domain::user::value_objects::UserId,
    ) -> mokkan_core::domain::errors::DomainResult<Option<mokkan_core::domain::user::entity::User>> {
        Ok(None)
    }
    async fn update(
        &self,
        _update: mokkan_core::domain::user::entity::UserUpdate,
    ) -> mokkan_core::domain::errors::DomainResult<mokkan_core::domain::user::entity::User> {
        Err(mokkan_core::domain::errors::DomainError::NotFound("not implemented".into()))
    }
    async fn list_page(
        &self,
        _limit: u32,
        _cursor: Option<mokkan_core::domain::user::value_objects::UserListCursor>,
        _search: Option<&str>,
    ) -> mokkan_core::domain::errors::DomainResult<(
        Vec<mokkan_core::domain::user::entity::User>,
        Option<mokkan_core::domain::user::value_objects::UserListCursor>,
    )> {
        Ok((vec![], None))
    }
}

/* ------------------------------ Article repos -------------------------- */

pub struct DummyArticleWrite;

#[async_trait]
impl mokkan_core::domain::article::repository::ArticleWriteRepository for DummyArticleWrite {
    async fn insert(
        &self,
        _new: mokkan_core::domain::article::entity::NewArticle,
    ) -> mokkan_core::domain::errors::DomainResult<mokkan_core::domain::article::entity::Article> {
        Err(mokkan_core::domain::errors::DomainError::NotFound("not implemented".into()))
    }
    async fn update(
        &self,
        _article: mokkan_core::domain::article::entity::ArticleUpdate,
    ) -> mokkan_core::domain::errors::DomainResult<mokkan_core::domain::article::entity::Article> {
        Err(mokkan_core::domain::errors::DomainError::NotFound("not implemented".into()))
    }
    async fn delete(
        &self,
        _id: mokkan_core::domain::article::value_objects::ArticleId,
    ) -> mokkan_core::domain::errors::DomainResult<()> {
        Ok(())
    }
}

pub struct DummyArticleRead;

#[async_trait]
impl mokkan_core::domain::article::repository::ArticleReadRepository for DummyArticleRead {
    async fn find_by_id(
        &self,
        _id: mokkan_core::domain::article::value_objects::ArticleId,
    ) -> mokkan_core::domain::errors::DomainResult<Option<mokkan_core::domain::article::entity::Article>> {
        Ok(None)
    }
    async fn find_by_slug(
        &self,
        _slug: &mokkan_core::domain::article::value_objects::ArticleSlug,
    ) -> mokkan_core::domain::errors::DomainResult<Option<mokkan_core::domain::article::entity::Article>> {
        Ok(None)
    }
    async fn list_page(
        &self,
        _include_drafts: bool,
        _limit: u32,
        _cursor: Option<mokkan_core::domain::article::value_objects::ArticleListCursor>,
        _search: Option<&str>,
    ) -> mokkan_core::domain::errors::DomainResult<(
        Vec<mokkan_core::domain::article::entity::Article>,
        Option<mokkan_core::domain::article::value_objects::ArticleListCursor>,
    )> {
        Ok((vec![], None))
    }
}

pub struct DummyArticleRevision;

#[async_trait]
impl mokkan_core::domain::article::repository::ArticleRevisionRepository for DummyArticleRevision {
    async fn append(
        &self,
        _article: &mokkan_core::domain::article::entity::Article,
        _edited_by: Option<mokkan_core::domain::user::value_objects::UserId>,
    ) -> mokkan_core::domain::errors::DomainResult<()> {
        Ok(())
    }
    async fn list_by_article(
        &self,
        _article_id: mokkan_core::domain::article::value_objects::ArticleId,
    ) -> mokkan_core::domain::errors::DomainResult<
        Vec<mokkan_core::domain::article::revision::ArticleRevision>
    > {
        Ok(vec![])
    }
}