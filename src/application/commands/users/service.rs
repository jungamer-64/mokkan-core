use std::sync::Arc;

use crate::application::ports::{
    refresh_token::Codec,
    security::{PasswordHasher, TokenManager},
    session_revocation::{Ports, Store},
    time::Clock,
};
use crate::domain::UserRepository;

#[must_use]
pub struct UserCommandService {
    pub(super) user_repo: Arc<dyn UserRepository>,
    pub(super) password_hasher: Arc<dyn PasswordHasher>,
    pub(super) token_manager: Arc<dyn TokenManager>,
    pub(super) refresh_token_codec: Arc<dyn Codec>,
    pub(super) session_stores: Ports,
    pub(super) clock: Arc<dyn Clock>,
}

impl UserCommandService {
    pub fn new(
        user_repo: Arc<dyn UserRepository>,
        password_hasher: Arc<dyn PasswordHasher>,
        token_manager: Arc<dyn TokenManager>,
        refresh_token_codec: Arc<dyn Codec>,
        session_revocation_store: Arc<dyn Store>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            user_repo,
            password_hasher,
            token_manager,
            refresh_token_codec,
            session_stores: Ports::from_store(session_revocation_store),
            clock,
        }
    }
}
