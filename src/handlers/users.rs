use axum::{extract::State, http::StatusCode, Json};
use uuid::Uuid;

use crate::error::DbError;
use crate::models::users::{NewUser, User};

pub async fn all_users() -> () {
    todo!()
}

// POST /api/user
pub async fn create_user() -> () {
    todo!()
}
