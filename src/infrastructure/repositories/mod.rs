// src/infrastructure/repositories/mod.rs
mod postgres_article;
mod postgres_user;

pub use postgres_article::{PostgresArticleReadRepository, PostgresArticleWriteRepository};
pub use postgres_user::PostgresUserRepository;
