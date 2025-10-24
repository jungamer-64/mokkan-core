// tests/support/mocks/article_repos.rs
use async_trait::async_trait;

/* -------------------------------- ArticleWriteRepository -------------------------------- */

/// ダミーの記事書き込みリポジトリ
pub struct DummyArticleWrite;

#[async_trait]
impl mokkan_core::domain::article::repository::ArticleWriteRepository for DummyArticleWrite {
    async fn insert(
        &self,
        _new: mokkan_core::domain::article::entity::NewArticle,
    ) -> mokkan_core::domain::errors::DomainResult<mokkan_core::domain::article::entity::Article> {
        Err(mokkan_core::domain::errors::DomainError::NotFound("not implemented".into()))
    }

    async fn update(
        &self,
        _article: mokkan_core::domain::article::entity::ArticleUpdate,
    ) -> mokkan_core::domain::errors::DomainResult<mokkan_core::domain::article::entity::Article> {
        Err(mokkan_core::domain::errors::DomainError::NotFound("not implemented".into()))
    }

    async fn delete(
        &self,
        _id: mokkan_core::domain::article::value_objects::ArticleId,
    ) -> mokkan_core::domain::errors::DomainResult<()> {
        Ok(())
    }
}

/* -------------------------------- ArticleReadRepository -------------------------------- */

/// ダミーの記事読み取りリポジトリ
pub struct DummyArticleRead;

#[async_trait]
impl mokkan_core::domain::article::repository::ArticleReadRepository for DummyArticleRead {
    async fn find_by_id(
        &self,
        _id: mokkan_core::domain::article::value_objects::ArticleId,
    ) -> mokkan_core::domain::errors::DomainResult<Option<mokkan_core::domain::article::entity::Article>> {
        Ok(None)
    }

    async fn find_by_slug(
        &self,
        _slug: &mokkan_core::domain::article::value_objects::ArticleSlug,
    ) -> mokkan_core::domain::errors::DomainResult<Option<mokkan_core::domain::article::entity::Article>> {
        Ok(None)
    }

    async fn list_page(
        &self,
        _include_drafts: bool,
        _limit: u32,
        _cursor: Option<mokkan_core::domain::article::value_objects::ArticleListCursor>,
        _search: Option<&str>,
    ) -> mokkan_core::domain::errors::DomainResult<(
        Vec<mokkan_core::domain::article::entity::Article>,
        Option<mokkan_core::domain::article::value_objects::ArticleListCursor>,
    )> {
        Ok((vec![], None))
    }
}

/* -------------------------------- ArticleRevisionRepository -------------------------------- */

/// ダミーの記事リビジョンリポジトリ
pub struct DummyArticleRevision;

#[async_trait]
impl mokkan_core::domain::article::repository::ArticleRevisionRepository for DummyArticleRevision {
    async fn append(
        &self,
        _article: &mokkan_core::domain::article::entity::Article,
        _edited_by: Option<mokkan_core::domain::user::value_objects::UserId>,
    ) -> mokkan_core::domain::errors::DomainResult<()> {
        Ok(())
    }

    async fn list_by_article(
        &self,
        _article_id: mokkan_core::domain::article::value_objects::ArticleId,
    ) -> mokkan_core::domain::errors::DomainResult<Vec<mokkan_core::domain::article::revision::ArticleRevision>> {
        Ok(vec![])
    }
}