use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::serde_time;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SessionInfoDto {
    pub session_id: String,
    pub user_agent: Option<String>,
    pub ip_address: Option<String>,
    #[serde(with = "serde_time")]
    pub created_at: DateTime<Utc>,
    pub revoked: bool,
}
