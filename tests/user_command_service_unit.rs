use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::{Duration, Utc};

mod support;

use mokkan_core::application::commands::users::{
    GrantRoleCommand, RevokeRoleCommand, UserCommandService,
};
use mokkan_core::application::dto::AuthenticatedUser;
use mokkan_core::domain::errors::DomainResult;
use mokkan_core::domain::user::entity::{NewUser, User, UserUpdate};
use mokkan_core::domain::user::repository::UserRepository;
use mokkan_core::domain::user::value_objects::{
    PasswordHash, Role, UserId, UserListCursor, Username,
};

struct InMemoryUserRepo {
    inner: Mutex<HashMap<i64, User>>,
}

impl InMemoryUserRepo {
    fn new(users: HashMap<i64, User>) -> Self {
        Self {
            inner: Mutex::new(users),
        }
    }
}

#[async_trait]
impl UserRepository for InMemoryUserRepo {
    async fn count(&self) -> DomainResult<u64> {
        let map = self.inner.lock().unwrap();
        Ok(map.len() as u64)
    }

    async fn insert(&self, _new_user: NewUser) -> DomainResult<User> {
        Err(mokkan_core::domain::errors::DomainError::NotFound(
            "not implemented".into(),
        ))
    }

    async fn find_by_username(&self, username: &Username) -> DomainResult<Option<User>> {
        let map = self.inner.lock().unwrap();
        for u in map.values() {
            if u.username.as_str() == username.as_str() {
                return Ok(Some(u.clone()));
            }
        }
        Ok(None)
    }

    async fn find_by_id(&self, id: UserId) -> DomainResult<Option<User>> {
        let map = self.inner.lock().unwrap();
        Ok(map.get(&i64::from(id)).cloned())
    }

    async fn update(&self, update: UserUpdate) -> DomainResult<User> {
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
        _cursor: Option<UserListCursor>,
        _search: Option<&str>,
    ) -> DomainResult<(Vec<User>, Option<UserListCursor>)> {
        Ok((vec![], None))
    }
}

#[tokio::test]
async fn grant_and_revoke_role_service_flow() {
    // prepare two users: admin (id=1) and target (id=2)
    let admin = User {
        id: UserId::new(1).unwrap(),
        username: Username::new("admin").unwrap(),
        password_hash: PasswordHash::new("hash".to_string()).unwrap(),
        role: Role::Admin,
        is_active: true,
        created_at: Utc::now(),
    };

    let target = User {
        id: UserId::new(2).unwrap(),
        username: Username::new("target").unwrap(),
        password_hash: PasswordHash::new("hash2".to_string()).unwrap(),
        role: Role::Author,
        is_active: true,
        created_at: Utc::now(),
    };

    let mut users = HashMap::new();
    users.insert(1, admin.clone());
    users.insert(2, target.clone());

    let repo = Arc::new(InMemoryUserRepo::new(users));

    // use test doubles for ports from support::mocks
    let password_hasher = Arc::new(support::DummyPasswordHasher);
    let token_manager = Arc::new(support::DummyTokenManager);
    let clock = Arc::new(support::DummyClock);

    let session_store = Arc::new(
        mokkan_core::infrastructure::security::session_store::InMemorySessionRevocationStore::new(),
    );
    let svc = UserCommandService::new(
        repo.clone(),
        password_hasher,
        token_manager,
        session_store,
        clock,
    );

    let actor = AuthenticatedUser {
        id: UserId::new(1).unwrap(),
        username: "admin".into(),
        role: Role::Admin,
        capabilities: Role::Admin.default_capabilities(),
        issued_at: Utc::now(),
        expires_at: Utc::now() + Duration::hours(1),
        session_id: None,
        token_version: None,
    };

    // grant admin role to target
    let grant_cmd = GrantRoleCommand {
        user_id: 2,
        role: Role::Admin,
    };
    let updated = svc
        .grant_role(&actor, grant_cmd)
        .await
        .expect("grant_role failed");
    assert_eq!(updated.role, Role::Admin);

    // now revoke (set back to Author)
    let revoke_cmd = RevokeRoleCommand { user_id: 2 };
    let updated2 = svc
        .revoke_role(&actor, revoke_cmd)
        .await
        .expect("revoke_role failed");
    assert_eq!(updated2.role, Role::Author);
}
