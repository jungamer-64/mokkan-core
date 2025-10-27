use crate::application::error::{ApplicationError, ApplicationResult};

pub(super) const MIN_PASSWORD_LENGTH: usize = 12;

pub(super) fn validate_password(password: &str) -> ApplicationResult<()> {
    if password.len() < MIN_PASSWORD_LENGTH {
        return Err(ApplicationError::validation(format!(
            "password must be at least {MIN_PASSWORD_LENGTH} characters"
        )));
    }

    let has_uppercase = password.chars().any(|c| c.is_uppercase());
    let has_lowercase = password.chars().any(|c| c.is_lowercase());
    let has_digit = password.chars().any(|c| c.is_ascii_digit());
    let has_special = password.chars().any(|c| !c.is_alphanumeric());

    if !(has_uppercase && has_lowercase && has_digit && has_special) {
        return Err(ApplicationError::validation(
            "password must contain uppercase, lowercase, digit, and special character",
        ));
    }

    Ok(())
}
