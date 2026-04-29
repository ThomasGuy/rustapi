use std::sync::Arc;

use axum::{extract::State, http::StatusCode, Json};
use diesel::prelude::*;
use serde::Deserialize;

use crate::auth::current_user::CurrentUser;
use crate::config::AppConfig;
use crate::db::{get_connection, DbConnection};
use crate::models::users::{NewUser, User};
use crate::schema::users;
use crate::utils::{hash_password, AppJson, AppResult, AppState, DbError};

#[derive(Debug, Deserialize)]
pub struct UserSignIn {
    username: String,
    email: String,
    password: String,
}

// POST /api/user
#[tracing::instrument(skip(state, payload), fields(user.email = %payload.email))]
pub async fn register(
    State(state): State<AppState>,
    AppJson(payload): AppJson<UserSignIn>,
) -> AppResult<(StatusCode, Json<User>)> {
    let mut conn: DbConnection = get_connection(&state.pool).await?;
    let hashed = hash_password(&payload.password)?;

    let new_user = NewUser {
        email: payload.email,
        username: payload.username,
        password_hash: hashed,
    };

    let user = diesel::insert_into(users::table)
        .values(&new_user)
        .get_result::<User>(&mut conn)
        .map_err(DbError::from)?;

    // 3. Tracing automatically includes user.email from the instrument macro
    tracing::info!(user_id = %user.id, "Successfully created user");

    Ok((StatusCode::CREATED, Json(user)))
}

// GET /api/user
pub async fn all_users(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> AppResult<Json<Vec<User>>> {
    let users_list = tokio::task::spawn_blocking(move || -> Result<Vec<User>, DbError> {
        let mut conn: DbConnection = state.pool.get().map_err(DbError::PoolError)?;
        users::table
            .load::<User>(&mut conn)
            .map_err(DbError::DatabaseError)
    })
    .await
    .map_err(DbError::from)??;

    Ok(Json(users_list))
}

pub async fn get_config(State(_config): State<Arc<AppConfig>>) -> () {
    todo!()
}
