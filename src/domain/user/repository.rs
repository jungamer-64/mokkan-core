// src/domain/user/repository.rs
use crate::domain::errors::DomainResult;
use crate::domain::user::{
    entity::{NewUser, User, UserUpdate},
    value_objects::{UserId, UserListCursor, Username},
};
use async_trait::async_trait;

#[async_trait]
pub trait UserRepository: Send + Sync {
    async fn count(&self) -> DomainResult<u64>;

    async fn insert(&self, new_user: NewUser) -> DomainResult<User>;

    async fn find_by_username(&self, username: &Username) -> DomainResult<Option<User>>;

    async fn find_by_id(&self, id: UserId) -> DomainResult<Option<User>>;

    async fn update(&self, update: UserUpdate) -> DomainResult<User>;

    async fn list_page(
        &self,
        limit: u32,
        cursor: Option<UserListCursor>,
        search: Option<&str>,
    ) -> DomainResult<(Vec<User>, Option<UserListCursor>)>;
}
