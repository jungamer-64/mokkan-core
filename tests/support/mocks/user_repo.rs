// tests/support/mocks/user_repo.rs
use async_trait::async_trait;

/// ダミーのユーザーリポジトリ（最小限の実装）
pub struct DummyUserRepo;

#[async_trait]
impl mokkan_core::domain::user::repository::UserRepository for DummyUserRepo {
    async fn count(&self) -> mokkan_core::domain::errors::DomainResult<u64> {
        Ok(0)
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
        _username: &mokkan_core::domain::user::value_objects::Username,
    ) -> mokkan_core::domain::errors::DomainResult<Option<mokkan_core::domain::user::entity::User>>
    {
        Ok(None)
    }

    async fn find_by_id(
        &self,
        _id: mokkan_core::domain::user::value_objects::UserId,
    ) -> mokkan_core::domain::errors::DomainResult<Option<mokkan_core::domain::user::entity::User>>
    {
        Ok(None)
    }

    async fn update(
        &self,
        _update: mokkan_core::domain::user::entity::UserUpdate,
    ) -> mokkan_core::domain::errors::DomainResult<mokkan_core::domain::user::entity::User> {
        Err(mokkan_core::domain::errors::DomainError::NotFound(
            "not implemented".into(),
        ))
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
