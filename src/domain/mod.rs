// src/domain/mod.rs
pub mod article;
pub mod audit;
pub mod errors;
pub mod user;

pub use article::entity::{Article, ArticleUpdate, NewArticle};
pub use article::repository::{
    ReadRepo as ArticleReadRepository, RevisionRepo as ArticleRevisionRepository,
    WriteRepo as ArticleWriteRepository,
};
pub use article::revision::{Parts as ArticleRevisionParts, Revision as ArticleRevision};
pub use article::value_objects::{
    ArticleBody, ArticleId, ArticleListCursor, ArticleSlug, ArticleTitle,
};
pub use user::entity::{NewUser, User, UserUpdate};
pub use user::repository::Repo as UserRepository;
pub use user::value_objects::{Capability, PasswordHash, Role, UserId, UserListCursor, Username};
