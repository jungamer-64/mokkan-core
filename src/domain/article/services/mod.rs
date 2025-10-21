// src/domain/article/services/mod.rs
use std::sync::Arc;

use chrono::Utc;

use crate::application::ports::util::SlugGenerator;
use crate::domain::article::repository::ArticleReadRepository;
use crate::domain::article::value_objects::{ArticleId, ArticleSlug, ArticleTitle};
use crate::domain::errors::DomainResult;

/// Domain service responsible for producing unique slugs for articles.
pub struct ArticleSlugService {
    read_repo: Arc<dyn ArticleReadRepository>,
    generator: Arc<dyn SlugGenerator>,
}

impl ArticleSlugService {
    pub fn new(
        read_repo: Arc<dyn ArticleReadRepository>,
        generator: Arc<dyn SlugGenerator>,
    ) -> Self {
        Self {
            read_repo,
            generator,
        }
    }

    pub async fn generate_unique_slug(
        &self,
        title: &ArticleTitle,
        ignore_id: Option<ArticleId>,
    ) -> DomainResult<ArticleSlug> {
        let base = self.generator.slugify(title.as_str());
        let base_slug = if base.is_empty() {
            format!("article-{}", Utc::now().timestamp())
        } else {
            base
        };

        let mut candidate = base_slug.clone();
        let mut counter = 1u64;

        loop {
            let slug = ArticleSlug::new(candidate.clone())?;
            match self.read_repo.find_by_slug(&slug).await? {
                Some(existing) if ignore_id.map(|id| id == existing.id).unwrap_or(false) => {
                    return Ok(slug);
                }
                Some(_) => {
                    candidate = format!("{}-{}", base_slug, counter);
                    counter += 1;
                }
                None => return Ok(slug),
            }
        }
    }
}
