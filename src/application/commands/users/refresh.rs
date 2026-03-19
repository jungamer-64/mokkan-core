use super::UserCommandService;
use crate::{
    application::{
        dto::{AuthTokenDto, TokenSubject},
        error::{ApplicationError, ApplicationResult},
        ports::refresh_token::RefreshTokenClaims,
    },
    domain::user::UserId,
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use uuid::Uuid;

enum ParsedRefreshToken {
    Signed {
        session_id: String,
        nonce: String,
        token_version: u32,
    },
    Legacy {
        user_id: UserId,
        session_id: String,
        nonce: String,
        token_version: u32,
    },
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
        let parsed = self.parse_refresh_token(token).await?;
        let (user_id, session_id, nonce, token_ver_in_token) = match parsed {
            ParsedRefreshToken::Signed {
                session_id,
                nonce,
                token_version,
            } => {
                let user_id = self.user_id_for_session(&session_id).await?;
                (user_id, session_id, nonce, token_version)
            }
            ParsedRefreshToken::Legacy {
                user_id,
                session_id,
                nonce,
                token_version,
            } => (user_id, session_id, nonce, token_version),
        };

        if self
            .session_revocation_store
            .is_revoked(&session_id)
            .await?
        {
            return Err(ApplicationError::forbidden("session revoked"));
        }

        let user = self
            .user_repo
            .find_by_id(user_id)
            .await?
            .ok_or_else(|| ApplicationError::not_found("user not found"))?;

        self.ensure_token_version_not_revoked(&user, token_ver_in_token)
            .await?;

        Ok((user, session_id, nonce, token_ver_in_token))
    }

    async fn user_id_for_session(&self, session_id: &str) -> ApplicationResult<UserId> {
        let meta = self
            .session_revocation_store
            .get_session_metadata(session_id)
            .await?
            .ok_or_else(|| ApplicationError::validation("invalid refresh token"))?;
        UserId::new(meta.user_id).map_err(Into::into)
    }

    async fn ensure_token_version_not_revoked(
        &self,
        user: &crate::domain::user::User,
        token_ver_in_token: u32,
    ) -> ApplicationResult<()> {
        if let Some(min_version) = self
            .session_revocation_store
            .get_min_token_version(i64::from(user.id))
            .await?
        {
            if token_ver_in_token < min_version {
                return Err(ApplicationError::forbidden("token version revoked"));
            }
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
            .session_revocation_store
            .compare_and_swap_session_refresh_nonce(session_id, expected_nonce, &new_nonce)
            .await?;

        if !swapped {
            let used = self
                .session_revocation_store
                .is_session_refresh_nonce_used(session_id, expected_nonce)
                .await?;

            if used {
                self.session_revocation_store
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
            .session_revocation_store
            .get_min_token_version(i64::from(user.id))
            .await?
            .unwrap_or(0);

        self.refresh_token_codec.encode(&RefreshTokenClaims {
            session_id: session_id.to_string(),
            nonce: nonce.to_string(),
            token_version: current_min,
        })
    }

    async fn parse_refresh_token(&self, token: &str) -> ApplicationResult<ParsedRefreshToken> {
        if self.refresh_token_codec.can_decode(token) {
            let claims = self.refresh_token_codec.decode(token)?;
            return Ok(ParsedRefreshToken::Signed {
                session_id: claims.session_id,
                nonce: claims.nonce,
                token_version: claims.token_version,
            });
        }

        let (user_id_part, session_id, nonce, token_ver_str) =
            Self::decode_legacy_refresh_token_raw(token)?;

        let uid: i64 = user_id_part
            .parse()
            .map_err(|_| ApplicationError::validation("invalid refresh token"))?;
        let user_id = UserId::new(uid)?;

        let token_ver: u32 = token_ver_str
            .parse()
            .map_err(|_| ApplicationError::validation("invalid refresh token"))?;

        Ok(ParsedRefreshToken::Legacy {
            user_id,
            session_id,
            nonce,
            token_version: token_ver,
        })
    }

    fn decode_legacy_refresh_token_raw(
        token: &str,
    ) -> ApplicationResult<(String, String, String, String)> {
        let decoded = URL_SAFE_NO_PAD
            .decode(token.as_bytes())
            .map_err(|_| ApplicationError::validation("invalid refresh token"))?;
        let decoded_str = String::from_utf8(decoded)
            .map_err(|_| ApplicationError::validation("invalid refresh token"))?;

        let parts: Vec<&str> = decoded_str.split(':').collect();
        if parts.len() != 4 {
            return Err(ApplicationError::validation("invalid refresh token"));
        }

        Ok((
            parts[0].to_string(),
            parts[1].to_string(),
            parts[2].to_string(),
            parts[3].to_string(),
        ))
    }
}
