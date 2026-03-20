use super::UserCommandService;
use crate::{
    application::{
        AuthTokenDto, TokenSubject, UserDto,
        error::{AppError, AppResult},
        random_id,
    },
    domain::Username,
};

pub struct LoginUserCommand {
    pub username: String,
    pub password: String,
}

pub struct LoginResult {
    pub token: AuthTokenDto,
    pub user: UserDto,
}

impl UserCommandService {
    /// Authenticate a user and issue a new session token pair.
    ///
    /// # Errors
    ///
    /// Returns an error if the username is invalid, credentials do not match,
    /// the account is disabled, or token/session persistence fails.
    pub async fn login(&self, command: LoginUserCommand) -> AppResult<LoginResult> {
        let username = Username::new(command.username)?;
        let user = self
            .find_and_authenticate_user(username, &command.password)
            .await?;

        let session_id = random_id::v4_string()?;

        let token = self.issue_session_tokens(&user, &session_id).await?;
        let user_dto: UserDto = user.into();

        Ok(LoginResult {
            token,
            user: user_dto,
        })
    }

    async fn issue_session_tokens(
        &self,
        user: &crate::domain::User,
        session_id: &str,
    ) -> AppResult<AuthTokenDto> {
        let capabilities = user.role.default_capabilities();

        let refresh_nonce = self.create_session_refresh_nonce(session_id).await?;

        let subject = TokenSubject {
            user_id: user.id,
            username: user.username.to_string(),
            role: user.role,
            capabilities: capabilities.clone(),
            session_id: Some(session_id.to_string()),
            token_version: None,
        };

        let mut token = self.token_manager.issue(subject).await?;

        let refresh_token = self
            .build_refresh_token_for_user(user, session_id, &refresh_nonce)
            .await?;
        token.refresh_token = Some(refresh_token);

        self.session_stores
            .session_metadata
            .add_session_for_user(i64::from(user.id), session_id)
            .await?;
        self.session_stores
            .session_metadata
            .set_session_metadata(
                i64::from(user.id),
                session_id,
                None,
                None,
                self.clock.now().timestamp(),
            )
            .await?;

        Ok(token)
    }

    async fn create_session_refresh_nonce(&self, session_id: &str) -> AppResult<String> {
        let refresh_nonce = random_id::v4_string()?;
        self.session_stores
            .refresh_nonces
            .set_session_refresh_nonce(session_id, &refresh_nonce)
            .await?;
        Ok(refresh_nonce)
    }

    async fn find_and_authenticate_user(
        &self,
        username: Username,
        password: &str,
    ) -> AppResult<crate::domain::User> {
        let user = self
            .user_repo
            .find_by_username(&username)
            .await?
            .ok_or_else(|| AppError::unauthorized("invalid credentials"))?;

        if !user.is_active {
            return Err(AppError::forbidden("account is disabled"));
        }

        self.password_hasher
            .verify(password, user.password_hash.as_str())
            .await?;

        Ok(user)
    }
}
