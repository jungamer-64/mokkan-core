// src/application/ports/mod.rs
pub mod security;
pub mod time;
pub mod util;
pub mod session_revocation;

// Type aliases to make port injection sites more descriptive and reduce `dyn` noise
pub type PasswordHasherPort = dyn security::PasswordHasher;
pub type TokenManagerPort = dyn security::TokenManager;
pub type ClockPort = dyn time::Clock;
pub type SlugGeneratorPort = dyn util::SlugGenerator;
