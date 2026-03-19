use super::UserCommandService;
use crate::{
    application::{
        dto::{AuthTokenDto, TokenSubject},
        error::{ApplicationError, ApplicationResult},
        ports::session_revocation::RefreshTokenRecord,
    },
    domain::user::UserId,
};
use uuid::Uuid;

struct ParsedRefreshToken {
    session_id: String,
    nonce: String,
    token_version: u32,
}

pub struct RefreshTokenCommand {
    pub token: String,
}

impl UserCommandService {
    pub async fn refresh_token(
        &self,
        command: RefreshTokenCommand,
    ) -> ApplicationResult<AuthTokenDto> {
        let (user, session_id, nonce, _token_ver) = self
            .validate_and_load_user_from_refresh_token(&command.token)
            .await?;

        let new_access = self
            .perform_refresh_for_user(&user, &session_id, &nonce)
            .await?;

        Ok(new_access)
    }

    async fn validate_and_load_user_from_refresh_token(
        &self,
        token: &str,
    ) -> ApplicationResult<(crate::domain::user::User, String, String, u32)> {
        let ParsedRefreshToken {
            session_id,
            nonce,
            token_version,
        } = self.parse_refresh_token(token).await?;
        let user_id = self.user_id_for_session(&session_id).await?;
        self.ensure_session_not_revoked(&session_id).await?;
        let user = self.load_user_for_refresh(user_id).await?;

        self.ensure_token_version_not_revoked(&user, token_version)
            .await?;

        Ok((user, session_id, nonce, token_version))
    }

    async fn user_id_for_session(&self, session_id: &str) -> ApplicationResult<UserId> {
        let meta = self
            .session_stores
            .session_metadata
            .get_session_metadata(session_id)
            .await?
            .ok_or_else(|| ApplicationError::validation("invalid refresh token"))?;
        UserId::new(meta.user_id).map_err(Into::into)
    }

    async fn ensure_session_not_revoked(&self, session_id: &str) -> ApplicationResult<()> {
        if self
            .session_stores
            .revocation
            .is_revoked(session_id)
            .await?
        {
            return Err(ApplicationError::forbidden("session revoked"));
        }

        Ok(())
    }

    async fn load_user_for_refresh(
        &self,
        user_id: UserId,
    ) -> ApplicationResult<crate::domain::user::User> {
        self.user_repo
            .find_by_id(user_id)
            .await?
            .ok_or_else(|| ApplicationError::not_found("user not found"))
    }

    async fn ensure_token_version_not_revoked(
        &self,
        user: &crate::domain::user::User,
        token_ver_in_token: u32,
    ) -> ApplicationResult<()> {
        if let Some(min_version) = self
            .session_stores
            .token_versions
            .get_min_token_version(i64::from(user.id))
            .await?
            && token_ver_in_token < min_version
        {
            return Err(ApplicationError::forbidden("token version revoked"));
        }

        Ok(())
    }

    async fn perform_refresh_for_user(
        &self,
        user: &crate::domain::user::User,
        session_id: &str,
        expected_nonce: &str,
    ) -> ApplicationResult<AuthTokenDto> {
        let new_nonce = Uuid::new_v4().to_string();
        let swapped = self
            .session_stores
            .refresh_nonces
            .compare_and_swap_session_refresh_nonce(session_id, expected_nonce, &new_nonce)
            .await?;

        if !swapped {
            let used = self
                .session_stores
                .refresh_nonces
                .is_session_refresh_nonce_used(session_id, expected_nonce)
                .await?;

            if used {
                self.session_stores
                    .revocation
                    .revoke_sessions_for_user(i64::from(user.id))
                    .await?;
                return Err(ApplicationError::forbidden("refresh token reused"));
            } else {
                return Err(ApplicationError::forbidden(
                    "refresh token invalid or rotated",
                ));
            }
        }

        let subject = self.make_token_subject(user, session_id);
        let mut new_access = self.token_manager.issue(subject).await?;

        let new_refresh_token = self
            .build_refresh_token_for_user(user, session_id, &new_nonce)
            .await?;

        new_access.refresh_token = Some(new_refresh_token);

        Ok(new_access)
    }

    fn make_token_subject(
        &self,
        user: &crate::domain::user::User,
        session_id: &str,
    ) -> TokenSubject {
        let capabilities = user.role.default_capabilities();
        TokenSubject {
            user_id: user.id,
            username: user.username.to_string(),
            role: user.role,
            capabilities: capabilities.clone(),
            session_id: Some(session_id.to_string()),
            token_version: None,
        }
    }

    pub(super) async fn build_refresh_token_for_user(
        &self,
        user: &crate::domain::user::User,
        session_id: &str,
        nonce: &str,
    ) -> ApplicationResult<String> {
        let current_min = self
            .session_stores
            .token_versions
            .get_min_token_version(i64::from(user.id))
            .await?
            .unwrap_or(0);

        let token_id = Uuid::new_v4().to_string();
        self.session_stores
            .opaque_refresh_tokens
            .store_refresh_token_record(
                &token_id,
                &RefreshTokenRecord {
                    session_id: session_id.to_string(),
                    nonce: nonce.to_string(),
                    token_version: current_min,
                },
            )
            .await?;

        self.refresh_token_codec.encode_opaque_handle(&token_id)
    }

    async fn load_opaque_refresh_token(
        &self,
        token_id: &str,
    ) -> ApplicationResult<ParsedRefreshToken> {
        let record = self
            .session_stores
            .opaque_refresh_tokens
            .get_refresh_token_record(token_id)
            .await?
            .ok_or_else(|| ApplicationError::validation("invalid refresh token"))?;

        Ok(ParsedRefreshToken {
            session_id: record.session_id,
            nonce: record.nonce,
            token_version: record.token_version,
        })
    }

    async fn parse_refresh_token(&self, token: &str) -> ApplicationResult<ParsedRefreshToken> {
        if self.refresh_token_codec.is_opaque_token(token) {
            let token_id = self.refresh_token_codec.decode_opaque_handle(token)?;
            return self.load_opaque_refresh_token(&token_id).await;
        }

        Err(ApplicationError::validation("invalid refresh token"))
    }
}
