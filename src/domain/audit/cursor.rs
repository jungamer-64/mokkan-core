use crate::domain::errors::{DomainError, DomainResult};
use chrono::{DateTime, Utc};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};

#[derive(Debug, Clone)]
pub struct AuditLogCursor {
    pub created_at: DateTime<Utc>,
    pub id: i64,
}

impl AuditLogCursor {
    pub fn new(created_at: DateTime<Utc>, id: i64) -> Self {
        Self { created_at, id }
    }

    pub fn encode(&self) -> String {
        let raw = format!("{}|{}", self.created_at.to_rfc3339(), self.id);
        URL_SAFE_NO_PAD.encode(raw.as_bytes())
    }

    pub fn decode(token: &str) -> DomainResult<Self> {
        let bytes = URL_SAFE_NO_PAD
            .decode(token)
            .map_err(|_| DomainError::Validation("invalid cursor token".into()))?;
        let raw = String::from_utf8(bytes)
            .map_err(|_| DomainError::Validation("invalid cursor token".into()))?;
        let mut parts = raw.splitn(2, '|');
        let created_at_s = parts.next().ok_or_else(|| DomainError::Validation("invalid cursor token".into()))?;
        let id_s = parts.next().ok_or_else(|| DomainError::Validation("invalid cursor token".into()))?;
        let created_at = DateTime::parse_from_rfc3339(created_at_s)
            .map_err(|_| DomainError::Validation("invalid cursor token".into()))?
            .with_timezone(&Utc);
        let id = id_s.parse::<i64>().map_err(|_| DomainError::Validation("invalid cursor token".into()))?;
        Ok(Self::new(created_at, id))
    }
}
