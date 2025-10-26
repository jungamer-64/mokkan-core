// src/application/ports/mod.rs
pub mod security;
pub mod time;
pub mod util;
pub mod session_revocation;
pub mod authorization_code;

// Type aliases to make port injection sites more descriptive and reduce `dyn` noise
pub type PasswordHasherPort = dyn security::PasswordHasher;
pub type TokenManagerPort = dyn security::TokenManager;
pub type ClockPort = dyn time::Clock;
pub type SlugGeneratorPort = dyn util::SlugGenerator;
pub type AuthorizationCodeStorePort = dyn authorization_code::AuthorizationCodeStore;
