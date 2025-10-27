use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::Duration;

mod support;

use mokkan_core::application::commands::users::{
    LoginUserCommand, RefreshTokenCommand, UserCommandService,
};
use mokkan_core::application::ports::session_revocation::SessionRevocationStore;
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
        let expires_at = issued_at + Duration::hours(1);
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

#[tokio::test]
async fn refresh_token_reuse_triggers_revocation_in_memory() {
    // prepare a user
    let user = User {
        id: UserId::new(300).unwrap(),
        username: Username::new("reuse_user").unwrap(),
        password_hash: PasswordHash::new("hash".to_string()).unwrap(),
        role: Role::Author,
        is_active: true,
        created_at: chrono::Utc::now(),
    };

    let mut users = HashMap::new();
    users.insert(300, user.clone());

    // simple in-memory user repo
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
        ) -> mokkan_core::domain::errors::DomainResult<mokkan_core::domain::user::entity::User>
        {
            Err(mokkan_core::domain::errors::DomainError::NotFound(
                "not implemented".into(),
            ))
        }

        async fn find_by_username(
            &self,
            username: &mokkan_core::domain::user::value_objects::Username,
        ) -> mokkan_core::domain::errors::DomainResult<
            Option<mokkan_core::domain::user::entity::User>,
        > {
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
        ) -> mokkan_core::domain::errors::DomainResult<
            Option<mokkan_core::domain::user::entity::User>,
        > {
            let map = self.inner.lock().unwrap();
            Ok(map.get(&i64::from(id)).cloned())
        }

        async fn update(
            &self,
            update: mokkan_core::domain::user::entity::UserUpdate,
        ) -> mokkan_core::domain::errors::DomainResult<mokkan_core::domain::user::entity::User>
        {
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

    let repo = Arc::new(InMemoryUserRepo::new(users));
    let password_hasher = Arc::new(support::DummyPasswordHasher);
    let token_manager = Arc::new(FakeTokenManager::default());
    let clock = Arc::new(support::DummyClock);
    let session_store = Arc::new(
        mokkan_core::infrastructure::security::session_store::InMemorySessionRevocationStore::new(),
    );

    let svc = Arc::new(UserCommandService::new(
        repo,
        password_hasher,
        token_manager,
        session_store.clone(),
        clock,
    ));

    // login to get a refresh token
    let login = svc
        .login(LoginUserCommand {
            username: "reuse_user".into(),
            password: "pwd".into(),
        })
        .await
        .expect("login");
    let refresh_token = login.token.refresh_token.expect("refresh token returned");
    let session_id = login.token.session_id.expect("session id");

    // first refresh should succeed
    let r1 = svc
        .refresh_token(RefreshTokenCommand {
            token: refresh_token.clone(),
        })
        .await;
    assert!(r1.is_ok(), "first refresh should succeed");

    // reuse should trigger detection and revoke the session(s)
    let r2 = svc
        .refresh_token(RefreshTokenCommand {
            token: refresh_token.clone(),
        })
        .await;
    assert!(r2.is_err(), "reusing refresh token should fail");

    // the session should be revoked after reuse detection
    let revoked = session_store
        .is_revoked(&session_id)
        .await
        .expect("check revoked");
    assert!(revoked, "session should be revoked after reuse detection");
}
