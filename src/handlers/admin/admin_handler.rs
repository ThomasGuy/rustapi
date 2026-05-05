use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde::Serialize;
use uuid::Uuid;

use crate::db::{get_connection, DbConnection};
use crate::schema::users;
use crate::utils::{AppResult, AppState, DbError};
use crate::{auth::CurrentUser, utils::AppError};

#[derive(Serialize, Queryable, Selectable)]
#[diesel(table_name = crate::schema::users)]
#[serde(rename_all = "camelCase")]
pub struct UserResponse {
    pub id: Uuid,
    pub email: String,
    pub username: String,
    pub is_admin: bool,
    pub last_login_at: Option<NaiveDateTime>,
    pub created_at: NaiveDateTime,
}

//  GET /admin/users
pub async fn admin_users(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> AppResult<Json<Vec<UserResponse>>> {
    //  1. Guard
    if !user.is_admin {
        return Err(AppError::Forbidden("Admins only".into()));
    }

    let mut conn: DbConnection = get_connection(&state.pool).await?;

    let response = users::table
        .select(UserResponse::as_select()) // Only fetches the 6 columns above
        .order(users::created_at.desc())
        .load::<UserResponse>(&mut conn)
        .map_err(DbError::from)?;

    Ok(Json(response))
}

// DELETE /admin/user/{id}
pub async fn delete_user_admin(
    State(state): State<AppState>,  // Waiter brings the tray
    Path(target_id): Path<Uuid>,    // Waiter grabs the ID from the URL
    CurrentUser(user): CurrentUser, // Waiter checks the ID badge (JWT)
) -> AppResult<StatusCode> {
    //  1. Guard
    if !user.is_admin {
        return Err(AppError::Forbidden("Admins only".into()));
    }
    let mut conn: DbConnection = get_connection(&state.pool).await?;

    diesel::delete(users::table.filter(users::id.eq(target_id)))
        .execute(&mut conn)
        .map_err(DbError::from)?;

    Ok(StatusCode::OK)
}
