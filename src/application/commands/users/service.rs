use std::sync::Arc;

use crate::application::ports::{
    security::{PasswordHasher, TokenManager},
    session_revocation::SessionRevocationStore,
    time::Clock,
};
use crate::domain::user::UserRepository;

pub struct UserCommandService {
    pub(super) user_repo: Arc<dyn UserRepository>,
    pub(super) password_hasher: Arc<dyn PasswordHasher>,
    pub(super) token_manager: Arc<dyn TokenManager>,
    pub(super) session_revocation_store: Arc<dyn SessionRevocationStore>,
    pub(super) clock: Arc<dyn Clock>,
}

impl UserCommandService {
    pub fn new(
        user_repo: Arc<dyn UserRepository>,
        password_hasher: Arc<dyn PasswordHasher>,
        token_manager: Arc<dyn TokenManager>,
        session_revocation_store: Arc<dyn SessionRevocationStore>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            user_repo,
            password_hasher,
            token_manager,
            session_revocation_store,
            clock,
        }
    }
}
