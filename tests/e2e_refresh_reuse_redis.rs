#![allow(clippy::multiple_crate_versions)]

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

/// テスト用の極小 TokenManager（決定論的なアクセストークンを返す）
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

struct InMemoryUserRepo {
    inner: std::sync::Mutex<HashMap<i64, User>>,
}

impl InMemoryUserRepo {
    const fn new(users: HashMap<i64, User>) -> Self {
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
        let found = {
            let map = self.inner.lock().unwrap();
            map.values()
                .find(|u| u.username.as_str() == username.as_str())
                .cloned()
        };
        Ok(found)
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
        {
            let mut map = self.inner.lock().unwrap();
            let id = i64::from(update.id);
            match map.get_mut(&id) {
                Some(user) => {
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
                None => Err(mokkan_core::domain::errors::DomainError::NotFound(
                    "user not found".into(),
                )),
            }
        }
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

fn redis_url() -> String {
    env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".into())
}

fn extract_host_port(url: &str) -> String {
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
}

async fn ensure_redis_available(url: &str) -> bool {
    let host_port = extract_host_port(url);
    match tokio::time::timeout(
        Duration::from_secs(2),
        tokio::net::TcpStream::connect(host_port.clone()),
    )
    .await
    {
        Ok(Ok(_)) => true,
        Ok(Err(error)) => {
            eprintln!("Skipping Redis integration test (connect failed to {host_port}): {error}");
            false
        }
        Err(_) => {
            eprintln!("Skipping Redis integration test (connect timeout to {host_port})");
            false
        }
    }
}

fn build_user_command_service(
    session_store: Arc<
        dyn mokkan_core::application::ports::session_revocation::SessionRevocationStore,
    >,
) -> Arc<UserCommandService> {
    let user = User {
        id: UserId::new(200).unwrap(),
        username: Username::new("redis_user").unwrap(),
        password_hash: PasswordHash::new("hash".to_string()).unwrap(),
        role: Role::Author,
        is_active: true,
        created_at: chrono::Utc::now(),
    };

    let mut users = HashMap::new();
    users.insert(200, user);

    let repo: Arc<dyn mokkan_core::domain::user::repository::UserRepository> =
        Arc::new(InMemoryUserRepo::new(users));
    let password_hasher: Arc<dyn mokkan_core::application::ports::security::PasswordHasher> =
        Arc::new(support::mocks::DummyPasswordHasher);
    let token_manager: Arc<dyn mokkan_core::application::ports::security::TokenManager> =
        Arc::new(FakeTokenManager);
    let clock: Arc<dyn mokkan_core::application::ports::time::Clock> =
        Arc::new(support::mocks::DummyClock);

    Arc::new(UserCommandService::new(
        repo,
        password_hasher,
        token_manager,
        Arc::new(
            mokkan_core::infrastructure::security::refresh_token::HmacRefreshTokenCodec::new(
                "test-refresh-secret",
            )
            .expect("refresh token codec"),
        ),
        session_store,
        clock,
    ))
}

async fn login_for_refresh_token(svc: &UserCommandService, label: &str) -> String {
    svc.login(LoginUserCommand {
        username: "redis_user".into(),
        password: "pwd".into(),
    })
    .await
    .unwrap_or_else(|_| panic!("{label} failed"))
    .token
    .refresh_token
    .unwrap_or_else(|| panic!("{label} missing refresh token"))
}

async fn run_concurrent_refreshes(
    svc: Arc<UserCommandService>,
    refresh_token: String,
) -> (
    mokkan_core::application::ApplicationResult<mokkan_core::application::dto::AuthTokenDto>,
    mokkan_core::application::ApplicationResult<mokkan_core::application::dto::AuthTokenDto>,
) {
    let svc1 = Arc::clone(&svc);
    let token1 = refresh_token.clone();
    let h1 = tokio::spawn(async move {
        svc1.refresh_token(RefreshTokenCommand { token: token1 })
            .await
    });

    let svc2 = Arc::clone(&svc);
    let h2 = tokio::spawn(async move {
        svc2.refresh_token(RefreshTokenCommand {
            token: refresh_token,
        })
        .await
    });

    (
        h1.await.expect("task1 panicked"),
        h2.await.expect("task2 panicked"),
    )
}

/// Redis 必須の統合テスト。
/// ローカル/CI で Redis が起動していない場合は **スキップ** します。
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires a running Redis instance"]
async fn refresh_token_single_use_with_redis_store() {
    let url = redis_url();

    // CI 等で Redis 起動と競合しにくいように、わずかに待つ
    sleep(Duration::from_millis(200)).await;

    if !ensure_redis_available(&url).await {
        return;
    }

    let store =
        mokkan_core::infrastructure::security::redis_session_store::RedisSessionRevocationStore::from_url(&url)
            .expect("create redis store");
    let session_store =
        mokkan_core::infrastructure::security::redis_session_store::into_arc(store);
    let svc = build_user_command_service(session_store);

    // 1回目のログインで refresh token を取得
    let refresh_token = login_for_refresh_token(&svc, "login").await;

    // 1回目のリフレッシュは成功する
    let r1 = svc
        .refresh_token(RefreshTokenCommand {
            token: refresh_token.clone(),
        })
        .await;
    assert!(r1.is_ok(), "first refresh should succeed");

    // 同一 refresh token の再利用は失敗する
    let r2 = svc
        .refresh_token(RefreshTokenCommand {
            token: refresh_token.clone(),
        })
        .await;
    assert!(r2.is_err(), "reusing refresh token should fail");

    // 併走テスト用に新しい refresh token を取得
    let refresh_token2 = login_for_refresh_token(&svc, "login2").await;
    let (r1, r2) = run_concurrent_refreshes(Arc::clone(&svc), refresh_token2).await;

    let ok_count = usize::from(r1.is_ok()) + usize::from(r2.is_ok());
    assert_eq!(
        ok_count, 1,
        "exactly one concurrent refresh should succeed with Redis store"
    );
}
