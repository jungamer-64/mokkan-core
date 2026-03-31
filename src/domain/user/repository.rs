// src/domain/user/repository.rs
use crate::async_support::BoxFuture;
use crate::domain::errors::DomainResult;
use crate::domain::{NewUser, User, UserId, UserListCursor, UserUpdate, Username};

pub trait Repo: Send + Sync {
    fn count(&self) -> BoxFuture<'_, DomainResult<u64>>;

    fn insert(&self, new_user: NewUser) -> BoxFuture<'_, DomainResult<User>>;

    fn find_by_username<'a>(
        &'a self,
        username: &'a Username,
    ) -> BoxFuture<'a, DomainResult<Option<User>>>;

    fn find_by_id(&self, id: UserId) -> BoxFuture<'_, DomainResult<Option<User>>>;

    fn update(&self, update: UserUpdate) -> BoxFuture<'_, DomainResult<User>>;

    fn list_page<'a>(
        &'a self,
        limit: u32,
        cursor: Option<UserListCursor>,
        search: Option<&'a str>,
    ) -> BoxFuture<'a, DomainResult<(Vec<User>, Option<UserListCursor>)>>;
}
