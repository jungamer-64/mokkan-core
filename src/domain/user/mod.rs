// src/domain/user/mod.rs
pub mod entity;
pub mod repository;
pub mod value_objects;

pub use entity::{NewUser, User};
pub use repository::UserRepository;
pub use value_objects::{Capability, PasswordHash, Role, UserId, Username};
