// tests/support/mocks/article_repos.rs
use mokkan_core::async_support::{BoxFuture, boxed};

/* -------------------------------- ArticleWriteRepository -------------------------------- */

/// ダミーの記事書き込みリポジトリ
pub struct DummyArticleWrite;

impl mokkan_core::domain::ArticleWriteRepository for DummyArticleWrite {
    fn insert(
        &self,
        _new: mokkan_core::domain::article::entity::NewArticle,
    ) -> BoxFuture<
        '_,
        mokkan_core::domain::errors::DomainResult<mokkan_core::domain::article::entity::Article>,
    > {
        boxed(async move {
            Err(mokkan_core::domain::errors::DomainError::NotFound(
                "not implemented".into(),
            ))
        })
    }

    fn update(
        &self,
        _article: mokkan_core::domain::article::entity::ArticleUpdate,
    ) -> BoxFuture<
        '_,
        mokkan_core::domain::errors::DomainResult<mokkan_core::domain::article::entity::Article>,
    > {
        boxed(async move {
            Err(mokkan_core::domain::errors::DomainError::NotFound(
                "not implemented".into(),
            ))
        })
    }

    fn delete(
        &self,
        _id: mokkan_core::domain::article::value_objects::ArticleId,
    ) -> BoxFuture<'_, mokkan_core::domain::errors::DomainResult<()>> {
        boxed(async move { Ok(()) })
    }
}

/* -------------------------------- ArticleReadRepository -------------------------------- */

/// ダミーの記事読み取りリポジトリ
pub struct DummyArticleRead;

impl mokkan_core::domain::ArticleReadRepository for DummyArticleRead {
    fn find_by_id(
        &self,
        _id: mokkan_core::domain::article::value_objects::ArticleId,
    ) -> BoxFuture<
        '_,
        mokkan_core::domain::errors::DomainResult<
            Option<mokkan_core::domain::article::entity::Article>,
        >,
    > {
        boxed(async move { Ok(None) })
    }

    fn find_by_slug<'a>(
        &'a self,
        _slug: &mokkan_core::domain::article::value_objects::ArticleSlug,
    ) -> BoxFuture<
        'a,
        mokkan_core::domain::errors::DomainResult<
            Option<mokkan_core::domain::article::entity::Article>,
        >,
    > {
        boxed(async move { Ok(None) })
    }

    fn list_page<'a>(
        &'a self,
        _include_drafts: bool,
        _limit: u32,
        _cursor: Option<mokkan_core::domain::article::value_objects::ArticleListCursor>,
        _search: Option<&'a str>,
    ) -> BoxFuture<
        'a,
        mokkan_core::domain::errors::DomainResult<(
            Vec<mokkan_core::domain::article::entity::Article>,
            Option<mokkan_core::domain::article::value_objects::ArticleListCursor>,
        )>,
    > {
        boxed(async move { Ok((vec![], None)) })
    }
}

/* -------------------------------- ArticleRevisionRepository -------------------------------- */

/// ダミーの記事リビジョンリポジトリ
pub struct DummyArticleRevision;

impl mokkan_core::domain::ArticleRevisionRepository for DummyArticleRevision {
    fn append<'a>(
        &'a self,
        _article: &mokkan_core::domain::article::entity::Article,
        _edited_by: Option<mokkan_core::domain::user::value_objects::UserId>,
    ) -> BoxFuture<'a, mokkan_core::domain::errors::DomainResult<()>> {
        boxed(async move { Ok(()) })
    }

    fn list_by_article(
        &self,
        _article_id: mokkan_core::domain::article::value_objects::ArticleId,
    ) -> BoxFuture<
        '_,
        mokkan_core::domain::errors::DomainResult<Vec<mokkan_core::domain::ArticleRevision>>,
    > {
        boxed(async move { Ok(vec![]) })
    }
}
