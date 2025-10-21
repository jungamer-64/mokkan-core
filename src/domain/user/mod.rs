// src/domain/user/mod.rs
pub mod entity;
pub mod repository;
pub mod value_objects;

pub use entity::{NewUser, User, UserUpdate};
pub use repository::UserRepository;
pub use value_objects::{Capability, PasswordHash, Role, UserId, UserListCursor, Username};
