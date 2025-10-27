use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use std::env;
use tokio::time::{Duration, sleep};

mod support;

use mokkan_core::application::commands::users::{
    LoginUserCommand, RefreshTokenCommand, UserCommandService,
};
use mokkan_core::domain::user::entity::User;
use mokkan_core::domain::user::value_objects::{PasswordHash, Role, UserId, Username};

/// A tiny fake TokenManager used for tests which returns deterministic access tokens.
#[derive(Clone, Debug, Default)]
struct FakeTokenManager;

#[async_trait]
impl mokkan_core::application::ports::security::TokenManager for FakeTokenManager {
    async fn issue(
        &self,
        subject: mokkan_core::application::dto::TokenSubject,
    ) -> mokkan_core::application::ApplicationResult<mokkan_core::application::dto::AuthTokenDto>
    {
        let issued_at = chrono::Utc::now();
        let expires_at = issued_at + chrono::Duration::hours(1);
        let expires_in = (expires_at.signed_duration_since(issued_at)).num_seconds();
        let sid = subject.session_id.clone();
        Ok(mokkan_core::application::dto::AuthTokenDto {
            token: format!(
                "access-{}-{}",
                i64::from(subject.user_id),
                sid.clone().unwrap_or_default()
            ),
            issued_at,
            expires_at,
            expires_in,
            session_id: sid,
            refresh_token: None,
        })
    }

    async fn authenticate(
        &self,
        _token: &str,
    ) -> mokkan_core::application::ApplicationResult<mokkan_core::application::dto::AuthenticatedUser>
    {
        Err(
            mokkan_core::application::error::ApplicationError::unauthorized(
                "not implemented for test",
            ),
        )
    }

    async fn public_jwk(&self) -> mokkan_core::application::ApplicationResult<serde_json::Value> {
        Ok(serde_json::json!({"keys":[]}))
    }
}

// A simple in-memory user repo used by the Redis integration test. Kept at
// module level to avoid long test function bodies and satisfy static analysis
// metrics.
struct InMemoryUserRepo {
    inner: std::sync::Mutex<HashMap<i64, User>>,
}

impl InMemoryUserRepo {
    fn new(users: HashMap<i64, User>) -> Self {
        Self {
            inner: std::sync::Mutex::new(users),
        }
    }
}

#[async_trait]
impl mokkan_core::domain::user::repository::UserRepository for InMemoryUserRepo {
    async fn count(&self) -> mokkan_core::domain::errors::DomainResult<u64> {
        let map = self.inner.lock().unwrap();
        Ok(map.len() as u64)
    }

    async fn insert(
        &self,
        _new_user: mokkan_core::domain::user::entity::NewUser,
    ) -> mokkan_core::domain::errors::DomainResult<mokkan_core::domain::user::entity::User> {
        Err(mokkan_core::domain::errors::DomainError::NotFound(
            "not implemented".into(),
        ))
    }

    async fn find_by_username(
        &self,
        username: &mokkan_core::domain::user::value_objects::Username,
    ) -> mokkan_core::domain::errors::DomainResult<Option<mokkan_core::domain::user::entity::User>>
    {
        let map = self.inner.lock().unwrap();
        for u in map.values() {
            if u.username.as_str() == username.as_str() {
                return Ok(Some(u.clone()));
            }
        }
        Ok(None)
    }

    async fn find_by_id(
        &self,
        id: mokkan_core::domain::user::value_objects::UserId,
    ) -> mokkan_core::domain::errors::DomainResult<Option<mokkan_core::domain::user::entity::User>>
    {
        let map = self.inner.lock().unwrap();
        Ok(map.get(&i64::from(id)).cloned())
    }

    async fn update(
        &self,
        update: mokkan_core::domain::user::entity::UserUpdate,
    ) -> mokkan_core::domain::errors::DomainResult<mokkan_core::domain::user::entity::User> {
        let mut map = self.inner.lock().unwrap();
        let id = i64::from(update.id);
        let user = map.get_mut(&id).ok_or_else(|| {
            mokkan_core::domain::errors::DomainError::NotFound("user not found".into())
        })?;

        if let Some(is_active) = update.is_active {
            user.is_active = is_active;
        }
        if let Some(role) = update.role {
            user.role = role;
        }
        if let Some(password_hash) = update.password_hash {
            user.password_hash = password_hash;
        }

        Ok(user.clone())
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

// Helper to check whether Redis is reachable at the given URL. Kept separate
// so the test body stays short for static analysis.
async fn redis_available(url: &str) -> bool {
    let host_port = {
        let mut s = url;
        if let Some(i) = s.find("://") {
            s = &s[i + 3..];
        }
        if let Some(i) = s.rfind('/') {
            s = &s[..i];
        }
        if let Some(i) = s.rfind('@') {
            s = &s[i + 1..];
        }
        s.to_string()
    };

    match tokio::time::timeout(
        Duration::from_secs(2),
        tokio::net::TcpStream::connect(host_port.clone()),
    )
    .await
    {
        Ok(Ok(_)) => true,
        _ => false,
    }
}

#[tokio::test]
async fn refresh_token_single_use_with_redis_store() {
    // Use REDIS_URL env if set, otherwise default to localhost:6379
    let url = env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".into());

    // Give a small grace period in case the test runner starts Redis concurrently
    sleep(Duration::from_millis(200)).await;
    // Quick connectivity check: skip the test if Redis is not reachable.
    if !redis_available(&url).await {
        eprintln!("Skipping Redis integration test because Redis is not reachable");
        return;
    }

    // Build a Redis-backed session revocation store
    let store = mokkan_core::infrastructure::security::redis_session_store::RedisSessionRevocationStore::from_url(&url)
        .expect("create redis store");
    let session_store = mokkan_core::infrastructure::security::redis_session_store::into_arc(store);

    // prepare a user
    let user = User {
        id: UserId::new(200).unwrap(),
        username: Username::new("redis_user").unwrap(),
        password_hash: PasswordHash::new("hash".to_string()).unwrap(),
        role: Role::Author,
        is_active: true,
        created_at: chrono::Utc::now(),
    };

    let mut users = HashMap::new();
    users.insert(200, user.clone());

    // A simple in-memory user repo for this test
    // InMemoryUserRepo is defined below at module-level to keep this test function short.

    let repo: Arc<dyn mokkan_core::domain::user::repository::UserRepository> =
        Arc::new(InMemoryUserRepo::new(users));
    let password_hasher: Arc<dyn mokkan_core::application::ports::security::PasswordHasher> =
        Arc::new(support::mocks::DummyPasswordHasher);
    let token_manager: Arc<dyn mokkan_core::application::ports::security::TokenManager> =
        Arc::new(FakeTokenManager::default());
    let clock: Arc<dyn mokkan_core::application::ports::time::Clock> =
        Arc::new(support::mocks::DummyClock);

    let svc = Arc::new(UserCommandService::new(
        repo,
        password_hasher,
        token_manager,
        session_store,
        clock,
    ));

    // login to get a refresh token
    let login = svc
        .login(LoginUserCommand {
            username: "redis_user".into(),
            password: "pwd".into(),
        })
        .await
        .expect("login");
    let refresh_token = login.token.refresh_token.expect("refresh token returned");

    // first refresh should succeed
    let r1 = svc
        .refresh_token(RefreshTokenCommand {
            token: refresh_token.clone(),
        })
        .await;
    assert!(r1.is_ok(), "first refresh should succeed");

    // reuse should fail
    let r2 = svc
        .refresh_token(RefreshTokenCommand {
            token: refresh_token.clone(),
        })
        .await;
    assert!(r2.is_err(), "reusing refresh token should fail");

    // fresh login for concurrency test
    let login2 = svc
        .login(LoginUserCommand {
            username: "redis_user".into(),
            password: "pwd".into(),
        })
        .await
        .expect("login2");
    let refresh_token2 = login2.token.refresh_token.expect("refresh token 2");

    // spawn two concurrent refresh attempts with the same token
    let svc1 = svc.clone();
    let tkn = refresh_token2.clone();
    let h1 = tokio::spawn(async move {
        svc1.refresh_token(RefreshTokenCommand { token: tkn.clone() })
            .await
    });

    let svc2 = svc.clone();
    let tkn2 = refresh_token2.clone();
    let h2 = tokio::spawn(async move {
        svc2.refresh_token(RefreshTokenCommand {
            token: tkn2.clone(),
        })
        .await
    });

    let r1 = h1.await.expect("task1 panicked");
    let r2 = h2.await.expect("task2 panicked");

    let ok_count = [r1, r2].iter().filter(|r| r.is_ok()).count();
    assert_eq!(
        ok_count, 1,
        "exactly one concurrent refresh should succeed with Redis store"
    );
}
