use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(bound(
    serialize = "T: Serialize",
    deserialize = "T: serde::de::DeserializeOwned"
))]
pub struct CursorPage<T> {
    pub items: Vec<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
    pub has_more: bool,
}

impl<T> CursorPage<T> {
    pub fn new(items: Vec<T>, next_cursor: Option<String>) -> Self {
        let has_more = next_cursor.is_some();
        Self {
            items,
            next_cursor,
            has_more,
        }
    }
}
