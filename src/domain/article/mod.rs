#![allow(clippy::module_name_repetitions)]

// src/domain/article/mod.rs
pub mod entity;
pub mod repository;
pub mod revision;
pub mod services;
pub mod specifications;
pub mod value_objects;

pub use entity::{Article, ArticleUpdate, NewArticle};
pub use repository::{ArticleReadRepository, ArticleRevisionRepository, ArticleWriteRepository};
pub use revision::{ArticleRevision, ArticleRevisionParts};
pub use value_objects::{ArticleBody, ArticleId, ArticleListCursor, ArticleSlug, ArticleTitle};
