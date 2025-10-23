// tests/support/mocks.rs
use std::collections::HashSet;

pub struct DummyUserRepo;
#[async_trait::async_trait]
impl mokkan_core::domain::user::repository::UserRepository for DummyUserRepo {
    async fn count(&self) -> mokkan_core::domain::errors::DomainResult<u64> { Ok(0) }
    async fn insert(&self, _new_user: mokkan_core::domain::user::entity::NewUser) -> mokkan_core::domain::errors::DomainResult<mokkan_core::domain::user::entity::User> { Err(mokkan_core::domain::errors::DomainError::NotFound("not implemented".into())) }
    async fn find_by_username(&self, _username: &mokkan_core::domain::user::value_objects::Username) -> mokkan_core::domain::errors::DomainResult<Option<mokkan_core::domain::user::entity::User>> { Ok(None) }
    async fn find_by_id(&self, _id: mokkan_core::domain::user::value_objects::UserId) -> mokkan_core::domain::errors::DomainResult<Option<mokkan_core::domain::user::entity::User>> { Ok(None) }
    async fn update(&self, _update: mokkan_core::domain::user::entity::UserUpdate) -> mokkan_core::domain::errors::DomainResult<mokkan_core::domain::user::entity::User> { Err(mokkan_core::domain::errors::DomainError::NotFound("not implemented".into())) }
    async fn list_page(&self, _limit: u32, _cursor: Option<mokkan_core::domain::user::value_objects::UserListCursor>, _search: Option<&str>) -> mokkan_core::domain::errors::DomainResult<(Vec<mokkan_core::domain::user::entity::User>, Option<mokkan_core::domain::user::value_objects::UserListCursor>)> { Ok((vec![], None)) }
}

pub struct DummyArticleWrite;
#[async_trait::async_trait]
impl mokkan_core::domain::article::repository::ArticleWriteRepository for DummyArticleWrite {
    async fn insert(&self, _new: mokkan_core::domain::article::entity::NewArticle) -> mokkan_core::domain::errors::DomainResult<mokkan_core::domain::article::entity::Article> { Err(mokkan_core::domain::errors::DomainError::NotFound("not implemented".into())) }
    async fn update(&self, _article: mokkan_core::domain::article::entity::ArticleUpdate) -> mokkan_core::domain::errors::DomainResult<mokkan_core::domain::article::entity::Article> { Err(mokkan_core::domain::errors::DomainError::NotFound("not implemented".into())) }
    async fn delete(&self, _id: mokkan_core::domain::article::value_objects::ArticleId) -> mokkan_core::domain::errors::DomainResult<()> { Ok(()) }
}

pub struct DummyArticleRead;
#[async_trait::async_trait]
impl mokkan_core::domain::article::repository::ArticleReadRepository for DummyArticleRead {
    async fn find_by_id(&self, _id: mokkan_core::domain::article::value_objects::ArticleId) -> mokkan_core::domain::errors::DomainResult<Option<mokkan_core::domain::article::entity::Article>> { Ok(None) }
    async fn find_by_slug(&self, _slug: &mokkan_core::domain::article::value_objects::ArticleSlug) -> mokkan_core::domain::errors::DomainResult<Option<mokkan_core::domain::article::entity::Article>> { Ok(None) }
    async fn list_page(&self, _include_drafts: bool, _limit: u32, _cursor: Option<mokkan_core::domain::article::value_objects::ArticleListCursor>, _search: Option<&str>) -> mokkan_core::domain::errors::DomainResult<(Vec<mokkan_core::domain::article::entity::Article>, Option<mokkan_core::domain::article::value_objects::ArticleListCursor>)> { Ok((vec![], None)) }
}

pub struct DummyArticleRevision;
#[async_trait::async_trait]
impl mokkan_core::domain::article::repository::ArticleRevisionRepository for DummyArticleRevision {
    async fn append(&self, _article: &mokkan_core::domain::article::entity::Article, _edited_by: Option<mokkan_core::domain::user::value_objects::UserId>) -> mokkan_core::domain::errors::DomainResult<()> { Ok(()) }
    async fn list_by_article(&self, _article_id: mokkan_core::domain::article::value_objects::ArticleId) -> mokkan_core::domain::errors::DomainResult<Vec<mokkan_core::domain::article::revision::ArticleRevision>> { Ok(vec![]) }
}

pub struct DummyPasswordHasher;
#[async_trait::async_trait]
impl mokkan_core::application::ports::security::PasswordHasher for DummyPasswordHasher {
    async fn hash(&self, _password: &str) -> mokkan_core::application::ApplicationResult<String> { Ok("hash".into()) }
    async fn verify(&self, _password: &str, _expected_hash: &str) -> mokkan_core::application::ApplicationResult<()> { Ok(()) }
}

pub struct DummyTokenManager;
#[async_trait::async_trait]
impl mokkan_core::application::ports::security::TokenManager for DummyTokenManager {
    async fn issue(&self, _subject: mokkan_core::application::dto::TokenSubject) -> mokkan_core::application::ApplicationResult<mokkan_core::application::dto::AuthTokenDto> { Err(mokkan_core::application::error::ApplicationError::infrastructure("not implemented")) }
    async fn authenticate(&self, token: &str) -> mokkan_core::application::ApplicationResult<mokkan_core::application::dto::AuthenticatedUser> {
        if token == "test-token" {
            let now = chrono::Utc::now();
            let user = mokkan_core::application::dto::AuthenticatedUser {
                id: mokkan_core::domain::user::value_objects::UserId::new(1).unwrap(),
                username: "tester".into(),
                role: mokkan_core::domain::user::value_objects::Role::Admin,
                capabilities: HashSet::from([mokkan_core::domain::user::value_objects::Capability::new("audit","read")]),
                issued_at: now,
                expires_at: now + chrono::Duration::hours(1),
            };
            Ok(user)
        } else {
            Err(mokkan_core::application::error::ApplicationError::unauthorized("invalid token"))
        }
    }
}

pub struct DummyClock;
impl mokkan_core::application::ports::time::Clock for DummyClock {
    fn now(&self) -> chrono::DateTime<chrono::Utc> { chrono::Utc::now() }
}

pub struct DummySlug;
impl mokkan_core::application::ports::util::SlugGenerator for DummySlug {
    fn slugify(&self, s: &str) -> String { s.to_string() }
}

pub struct MockAuditRepo;
#[async_trait::async_trait]
impl mokkan_core::domain::audit::repository::AuditLogRepository for MockAuditRepo {
    async fn insert(&self, _log: mokkan_core::domain::audit::entity::AuditLog) -> mokkan_core::domain::errors::DomainResult<()> { Ok(()) }

    async fn list(&self, _limit: u32, _cursor: Option<mokkan_core::domain::audit::cursor::AuditLogCursor>) -> mokkan_core::domain::errors::DomainResult<(Vec<mokkan_core::domain::audit::entity::AuditLog>, Option<String>)> {
        let created_at = chrono::DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z").unwrap().with_timezone(&chrono::Utc);
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

    async fn find_by_user(&self, _user_id: i64, _limit: u32, _cursor: Option<mokkan_core::domain::audit::cursor::AuditLogCursor>) -> mokkan_core::domain::errors::DomainResult<(Vec<mokkan_core::domain::audit::entity::AuditLog>, Option<String>)> {
        self.list(_limit, _cursor).await
    }

    async fn find_by_resource(&self, _resource_type: &str, _resource_id: i64, _limit: u32, _cursor: Option<mokkan_core::domain::audit::cursor::AuditLogCursor>) -> mokkan_core::domain::errors::DomainResult<(Vec<mokkan_core::domain::audit::entity::AuditLog>, Option<String>)> {
        self.list(_limit, _cursor).await
    }
}

// Lightweight mock used by unit tests that need custom items
pub struct MockRepo {
    pub items: Vec<mokkan_core::domain::audit::entity::AuditLog>,
    pub next_cursor: Option<String>,
}

#[async_trait::async_trait]
impl mokkan_core::domain::audit::repository::AuditLogRepository for MockRepo {
    async fn insert(&self, _log: mokkan_core::domain::audit::entity::AuditLog) -> mokkan_core::domain::errors::DomainResult<()> {
        Ok(())
    }

    async fn list(&self, _limit: u32, _cursor: Option<mokkan_core::domain::audit::cursor::AuditLogCursor>) -> mokkan_core::domain::errors::DomainResult<(Vec<mokkan_core::domain::audit::entity::AuditLog>, Option<String>)> {
        Ok((self.items.clone(), self.next_cursor.clone()))
    }

    async fn find_by_user(&self, _user_id: i64, _limit: u32, _cursor: Option<mokkan_core::domain::audit::cursor::AuditLogCursor>) -> mokkan_core::domain::errors::DomainResult<(Vec<mokkan_core::domain::audit::entity::AuditLog>, Option<String>)> {
        Ok((self.items.clone(), self.next_cursor.clone()))
    }

    async fn find_by_resource(&self, _resource_type: &str, _resource_id: i64, _limit: u32, _cursor: Option<mokkan_core::domain::audit::cursor::AuditLogCursor>) -> mokkan_core::domain::errors::DomainResult<(Vec<mokkan_core::domain::audit::entity::AuditLog>, Option<String>)> {
        Ok((self.items.clone(), self.next_cursor.clone()))
    }
}
