use bcrypt;

use crate::utils::app_error::AppError;

pub fn hash_password(password: &str) -> Result<String, AppError> {
    bcrypt::hash(password, bcrypt::DEFAULT_COST)
        .map_err(|_| AppError::Internal("Failed to hash password".into()))
}

pub fn verify_password(password: &str, hash: &str) -> Result<bool, AppError> {
    bcrypt::verify(password, hash).map_err(|_| AppError::Auth("Invalid credentials".into()))
}
