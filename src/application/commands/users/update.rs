use super::{UserCommandService, capability::ensure_capability};
use crate::{
    application::{
        dto::{AuthenticatedUser, UserDto},
        error::{ApplicationError, ApplicationResult},
    },
    domain::user::{Role, UserId, UserUpdate},
};

pub struct UpdateUserCommand {
    pub user_id: i64,
    pub is_active: Option<bool>,
    pub role: Option<Role>,
}

impl UserCommandService {
    pub async fn update_user(
        &self,
        actor: &AuthenticatedUser,
        command: UpdateUserCommand,
    ) -> ApplicationResult<UserDto> {
        ensure_capability(actor, "users", "update")?;

        let user_id = UserId::new(command.user_id)?;

        if command.is_active.is_none() && command.role.is_none() {
            return Err(ApplicationError::validation(
                "at least one field must be provided",
            ));
        }

        let mut update = UserUpdate::new(user_id);

        if let Some(is_active) = command.is_active {
            update = update.with_is_active(is_active);
        }

        if let Some(role) = command.role {
            update = update.with_role(role);
        }

        let user = self.user_repo.update(update).await?;
        Ok(user.into())
    }
}
