use super::UserQueryService;
use crate::application::{
    dto::{AuthenticatedUser, UserProfileDto},
    error::{ApplicationError, ApplicationResult},
};

impl UserQueryService {
    /// Load the current authenticated user's profile.
    ///
    /// # Errors
    ///
    /// Returns an error if the backing user record no longer exists or the
    /// repository lookup fails.
    pub async fn get_profile(
        &self,
        actor: &AuthenticatedUser,
    ) -> ApplicationResult<UserProfileDto> {
        let user = self
            .user_repo
            .find_by_id(actor.id)
            .await?
            .ok_or_else(|| ApplicationError::not_found("user not found"))?;

        Ok(UserProfileDto::from_parts(user, actor))
    }
}
