use crate::domain::errors::DomainError;

const CNT_ARTICLE_SLUG: &str = "articles_slug_key";
const CNT_ARTICLE_AUTHOR: &str = "articles_author_id_fkey";
const CNT_ARTICLE_PUBLISHED_CHECK: &str = "articles_published_requires_timestamp_chk";
const CNT_USER_USERNAME: &str = "users_username_key";

pub fn map_sqlx(err: sqlx::Error) -> DomainError {
    match &err {
        sqlx::Error::Database(db_err) => {
            if let Some(constraint) = db_err.constraint() {
                return match constraint {
                    CNT_ARTICLE_SLUG => DomainError::Conflict("slug already exists".into()),
                    CNT_USER_USERNAME => DomainError::Conflict("username already exists".into()),
                    CNT_ARTICLE_AUTHOR => DomainError::NotFound("author not found".into()),
                    CNT_ARTICLE_PUBLISHED_CHECK => {
                        DomainError::Validation("published articles require published_at".into())
                    }
                    other => {
                        DomainError::Persistence(format!("database constraint violation: {other}"))
                    }
                };
            }

            if let Some(code) = db_err.code() {
                match code.as_ref() {
                    "23505" => {
                        return DomainError::Conflict("unique constraint violated".into());
                    }
                    "23503" => {
                        return DomainError::NotFound("referenced record not found".into());
                    }
                    "23514" => {
                        return DomainError::Validation("check constraint violated".into());
                    }
                    _ => {}
                }
            }

            DomainError::Persistence(db_err.message().to_string())
        }
        _ => DomainError::Persistence(err.to_string()),
    }
}
