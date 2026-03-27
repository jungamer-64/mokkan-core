use std::sync::Arc;

use chrono::{TimeZone, Utc};

use crate::application::{
    AppError, AppResult, AuthenticatedUser, SessionInfoDto,
    ports::{
        session_revocation::{Ports, Store},
        time::Clock,
    },
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListSessionsRequest {
    pub user_id: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevokeSessionRequest {
    pub session_id: String,
}

#[derive(Clone)]
pub struct SessionService {
    session_stores: Ports,
    clock: Arc<dyn Clock>,
}

impl SessionService {
    #[must_use]
    pub fn new(session_revocation_store: Arc<dyn Store>, clock: Arc<dyn Clock>) -> Self {
        Self {
            session_stores: Ports::from_store(session_revocation_store),
            clock,
        }
    }

    /// List sessions for a user and convert them into DTOs.
    ///
    /// # Errors
    ///
    /// Returns an error if the session metadata store cannot be queried.
    pub async fn list_sessions(
        &self,
        request: ListSessionsRequest,
    ) -> AppResult<Vec<SessionInfoDto>> {
        let infos = self
            .session_stores
            .session_metadata
            .list_sessions_for_user_with_meta(request.user_id)
            .await?;

        Ok(infos
            .into_iter()
            .map(|info| SessionInfoDto {
                session_id: info.session_id,
                user_agent: info.user_agent,
                ip_address: info.ip_address,
                created_at: self.created_at_from_unix(info.created_at_unix),
                revoked: info.revoked,
            })
            .collect())
    }

    /// Revoke a session if the caller owns it or can manage users.
    ///
    /// # Errors
    ///
    /// Returns an error if the caller is not allowed to revoke the session or
    /// if backing store operations fail.
    pub async fn revoke_session(
        &self,
        actor: &AuthenticatedUser,
        request: RevokeSessionRequest,
    ) -> AppResult<()> {
        let is_owner = self
            .session_stores
            .session_metadata
            .list_sessions_for_user(i64::from(actor.id))
            .await?
            .contains(&request.session_id);

        if !is_owner && !actor.has_capability("users", "update") {
            return Err(AppError::forbidden("not authorized to revoke this session"));
        }

        self.session_stores
            .revocation
            .revoke(&request.session_id)
            .await?;

        if let Some(meta) = self
            .session_stores
            .session_metadata
            .get_session_metadata(&request.session_id)
            .await?
            && meta.user_id != 0
        {
            let _ = self
                .session_stores
                .session_metadata
                .remove_session_for_user(meta.user_id, &request.session_id)
                .await;
        }

        let _ = self
            .session_stores
            .session_metadata
            .delete_session_metadata(&request.session_id)
            .await;

        Ok(())
    }

    fn created_at_from_unix(&self, created_at_unix: i64) -> chrono::DateTime<Utc> {
        if created_at_unix > 0 {
            Utc.timestamp_opt(created_at_unix, 0)
                .single()
                .unwrap_or_else(|| self.clock.now())
        } else {
            self.clock.now()
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashSet, sync::Arc};

    use chrono::{DateTime, Utc};

    use super::{RevokeSessionRequest, SessionService};
    use crate::{
        application::{
            AppError, AuthenticatedUser,
            ports::{
                session_revocation::{Revocation, SessionMetadataStore},
                time::Clock,
            },
        },
        domain::{Capability, Role, UserId, user::value_objects::Capability as UserCapability},
        infrastructure::security::session_store::InMemorySessionRevocationStore,
    };

    #[derive(Clone)]
    struct FixedClock(DateTime<Utc>);

    impl Clock for FixedClock {
        fn now(&self) -> DateTime<Utc> {
            self.0
        }
    }

    fn actor() -> AuthenticatedUser {
        let now = DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z")
            .expect("valid RFC3339")
            .with_timezone(&Utc);

        AuthenticatedUser {
            id: UserId::new(10).expect("user id"),
            username: "actor".into(),
            role: Role::Author,
            capabilities: HashSet::<Capability>::new(),
            issued_at: now,
            expires_at: now,
            session_id: None,
            token_version: None,
        }
    }

    #[tokio::test]
    async fn revoke_session_forbidden_for_other_user_without_capability() {
        let store = Arc::new(InMemorySessionRevocationStore::new());
        store
            .set_session_metadata(11, "sid-11", Some("ua"), Some("127.0.0.1"), 1)
            .await
            .expect("set metadata");

        let service = SessionService::new(
            store,
            Arc::new(FixedClock(
                DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z")
                    .expect("valid RFC3339")
                    .with_timezone(&Utc),
            )),
        );

        let err = service
            .revoke_session(
                &actor(),
                RevokeSessionRequest {
                    session_id: "sid-11".into(),
                },
            )
            .await
            .expect_err("revoke should be forbidden");

        assert!(
            matches!(err, AppError::Forbidden(msg) if msg == "not authorized to revoke this session")
        );
    }

    #[tokio::test]
    async fn revoke_session_allows_user_admin_capability() {
        let store = Arc::new(InMemorySessionRevocationStore::new());
        store
            .set_session_metadata(11, "sid-11", Some("ua"), Some("127.0.0.1"), 1)
            .await
            .expect("set metadata");

        let service = SessionService::new(
            store.clone(),
            Arc::new(FixedClock(
                DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z")
                    .expect("valid RFC3339")
                    .with_timezone(&Utc),
            )),
        );

        let mut actor = actor();
        actor
            .capabilities
            .insert(UserCapability::new("users", "update"));

        service
            .revoke_session(
                &actor,
                RevokeSessionRequest {
                    session_id: "sid-11".into(),
                },
            )
            .await
            .expect("admin capability should revoke session");

        assert!(store.is_revoked("sid-11").await.expect("is revoked"));
    }
}
