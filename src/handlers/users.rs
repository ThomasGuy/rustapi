use std::sync::Arc;

use axum::{extract::State, http::StatusCode, Json};

use diesel::prelude::*;

use crate::config::AppConfig;
use crate::db::{get_connection, DbConnection, DbPool};
use crate::models::users::{NewUser, User};
use crate::schema::users::dsl::*;
use crate::utils::error::{AppResult, DbError};

// POST /api/user
#[tracing::instrument(skip(pool, payload), fields(user.email = %payload.email))]
pub async fn create_user(
    State(pool): State<DbPool>,
    Json(payload): Json<NewUser>,
) -> AppResult<(StatusCode, Json<User>)> {
    let mut conn: DbConnection = get_connection(&pool).await?;

    let user = diesel::insert_into(users)
        .values(&payload)
        .get_result::<User>(&mut conn)
        .map_err(DbError::from)?;

    // 3. Tracing automatically includes user.email from the instrument macro
    tracing::info!(user_id = %user.id, "Successfully created user");

    Ok((StatusCode::CREATED, Json(user)))
}

// GET /api/user
pub async fn all_users(State(pool): State<DbPool>) -> AppResult<Json<Vec<User>>> {
    let users_list = tokio::task::spawn_blocking(move || -> Result<Vec<User>, DbError> {
        let mut conn: DbConnection = pool.get().map_err(DbError::PoolError)?;
        users
            .load::<User>(&mut conn)
            .map_err(DbError::DatabaseError)
    })
    .await??;

    Ok(Json(users_list))
}

pub async fn get_config(State(_config): State<Arc<AppConfig>>) -> () {
    todo!()
}
