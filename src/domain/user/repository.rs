use crate::domain::errors::DomainResult;
use crate::domain::user::{entity::{NewUser, User}, value_objects::{Username, UserId}};
use async_trait::async_trait;

#[async_trait]
pub trait UserRepository: Send + Sync {
    async fn count(&self) -> DomainResult<u64>;

    async fn insert(&self, new_user: NewUser) -> DomainResult<User>;

    async fn find_by_username(&self, username: &Username) -> DomainResult<Option<User>>;

    async fn find_by_id(&self, id: UserId) -> DomainResult<Option<User>>;
}
