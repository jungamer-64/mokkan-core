use super::UserCommandService;
use crate::{
    application::{
        dto::{AuthTokenDto, TokenSubject, UserDto},
        error::{ApplicationError, ApplicationResult},
    },
    domain::user::Username,
};
use uuid::Uuid;

pub struct LoginUserCommand {
    pub username: String,
    pub password: String,
}

pub struct LoginResult {
    pub token: AuthTokenDto,
    pub user: UserDto,
}

impl UserCommandService {
    pub async fn login(&self, command: LoginUserCommand) -> ApplicationResult<LoginResult> {
        let username = Username::new(command.username)?;
        let user = self
            .find_and_authenticate_user(username, &command.password)
            .await?;

        let session_id = Uuid::new_v4().to_string();

        let token = self.issue_session_tokens(&user, &session_id).await?;
        let user_dto: UserDto = user.into();

        Ok(LoginResult {
            token,
            user: user_dto,
        })
    }

    async fn issue_session_tokens(
        &self,
        user: &crate::domain::user::User,
        session_id: &str,
    ) -> ApplicationResult<AuthTokenDto> {
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

        self.session_revocation_store
            .add_session_for_user(i64::from(user.id), session_id)
            .await?;

        Ok(token)
    }

    async fn create_session_refresh_nonce(&self, session_id: &str) -> ApplicationResult<String> {
        let refresh_nonce = Uuid::new_v4().to_string();
        self.session_revocation_store
            .set_session_refresh_nonce(session_id, &refresh_nonce)
            .await?;
        Ok(refresh_nonce)
    }

    async fn find_and_authenticate_user(
        &self,
        username: Username,
        password: &str,
    ) -> ApplicationResult<crate::domain::user::User> {
        let user = self
            .user_repo
            .find_by_username(&username)
            .await?
            .ok_or_else(|| ApplicationError::unauthorized("invalid credentials"))?;

        if !user.is_active {
            return Err(ApplicationError::forbidden("account is disabled"));
        }

        self.password_hasher
            .verify(password, user.password_hash.as_str())
            .await?;

        Ok(user)
    }
}
