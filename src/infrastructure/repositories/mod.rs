// src/infrastructure/repositories/mod.rs
mod error;
mod postgres_article;
mod postgres_article_revision;
mod postgres_user;

pub(crate) use error::map_sqlx;
pub use postgres_article::{PostgresArticleReadRepository, PostgresArticleWriteRepository};
pub use postgres_article_revision::PostgresArticleRevisionRepository;
pub use postgres_user::PostgresUserRepository;
