use crate::domain::article::value_objects::ArticleId;
use crate::domain::user::UserId;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub enum ArticleEvent {
    Created {
        id: ArticleId,
        author_id: UserId,
        at: DateTime<Utc>,
    },
    Published {
        id: ArticleId,
        at: DateTime<Utc>,
    },
    Updated {
        id: ArticleId,
        at: DateTime<Utc>,
    },
}
