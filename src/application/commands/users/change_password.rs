use super::{UserCommandService, capability::ensure_capability, password::validate_password};
use crate::{
    application::{
        dto::AuthenticatedUser,
        error::{ApplicationError, ApplicationResult},
    },
    domain::user::{PasswordHash, UserId, UserUpdate},
};

pub struct ChangePasswordCommand {
    pub user_id: i64,
    pub current_password: Option<String>,
    pub new_password: String,
}

impl UserCommandService {
    pub async fn change_password(
        &self,
        actor: &AuthenticatedUser,
        command: ChangePasswordCommand,
    ) -> ApplicationResult<()> {
        let target_id = UserId::new(command.user_id)?;

        let user = self
            .user_repo
            .find_by_id(target_id)
            .await?
            .ok_or_else(|| ApplicationError::not_found("user not found"))?;

        self.verify_change_password_self(actor, &user, command.current_password.as_deref())
            .await?;

        self.validate_and_set_new_password(target_id, &command.new_password)
            .await?;

        Ok(())
    }

    async fn verify_change_password_self(
        &self,
        actor: &AuthenticatedUser,
        user: &crate::domain::user::User,
        current_password: Option<&str>,
    ) -> ApplicationResult<()> {
        let is_self = actor.id == user.id;

        if !is_self {
            ensure_capability(actor, "users", "update")?;
            return Ok(());
        }

        let current = current_password
            .ok_or_else(|| ApplicationError::validation("current password is required"))?;

        self.password_hasher
            .verify(current, user.password_hash.as_str())
            .await?;

        Ok(())
    }

    async fn validate_and_set_new_password(
        &self,
        target_id: UserId,
        new_password: &str,
    ) -> ApplicationResult<()> {
        validate_password(new_password)?;

        let hashed = self.password_hasher.hash(new_password).await?;
        let password_hash = PasswordHash::new(hashed)?;

        let update = UserUpdate::new(target_id).with_password_hash(password_hash);
        self.user_repo.update(update).await?;

        Ok(())
    }
}
