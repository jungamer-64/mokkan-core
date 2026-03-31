// tests/support/mocks/user_repo.rs
use mokkan_core::async_support::{BoxFuture, boxed};

/// ダミーのユーザーリポジトリ（最小限の実装）
pub struct DummyRepo;

impl mokkan_core::domain::UserRepository for DummyRepo {
    fn count(&self) -> BoxFuture<'_, mokkan_core::domain::errors::DomainResult<u64>> {
        boxed(async move { Ok(0) })
    }

    fn insert(
        &self,
        _new_user: mokkan_core::domain::user::entity::NewUser,
    ) -> BoxFuture<
        '_,
        mokkan_core::domain::errors::DomainResult<mokkan_core::domain::user::entity::User>,
    > {
        boxed(async move {
            Err(mokkan_core::domain::errors::DomainError::NotFound(
                "not implemented".into(),
            ))
        })
    }

    fn find_by_username<'a>(
        &'a self,
        _username: &mokkan_core::domain::user::value_objects::Username,
    ) -> BoxFuture<
        'a,
        mokkan_core::domain::errors::DomainResult<Option<mokkan_core::domain::user::entity::User>>,
    > {
        boxed(async move { Ok(None) })
    }

    fn find_by_id(
        &self,
        _id: mokkan_core::domain::user::value_objects::UserId,
    ) -> BoxFuture<
        '_,
        mokkan_core::domain::errors::DomainResult<Option<mokkan_core::domain::user::entity::User>>,
    > {
        boxed(async move { Ok(None) })
    }

    fn update(
        &self,
        _update: mokkan_core::domain::user::entity::UserUpdate,
    ) -> BoxFuture<
        '_,
        mokkan_core::domain::errors::DomainResult<mokkan_core::domain::user::entity::User>,
    > {
        boxed(async move {
            Err(mokkan_core::domain::errors::DomainError::NotFound(
                "not implemented".into(),
            ))
        })
    }

    fn list_page<'a>(
        &'a self,
        _limit: u32,
        _cursor: Option<mokkan_core::domain::user::value_objects::UserListCursor>,
        _search: Option<&'a str>,
    ) -> BoxFuture<
        'a,
        mokkan_core::domain::errors::DomainResult<(
            Vec<mokkan_core::domain::user::entity::User>,
            Option<mokkan_core::domain::user::value_objects::UserListCursor>,
        )>,
    > {
        boxed(async move { Ok((vec![], None)) })
    }
}
