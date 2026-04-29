use axum::{extract::State, Json};
use diesel::prelude::*;
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::current_user::CurrentUser;
use crate::db::{get_connection, DbConnection};
use crate::models::comments::{Comment, NewComment};
use crate::schema::comments;
use crate::utils::{AppJson, AppResult, AppState, DbError};

#[derive(Debug, Deserialize)]
pub struct NewCommentRequest {
    pub post_id: Uuid,
    pub comment: String,
}

pub async fn create_comment(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    AppJson(payload): AppJson<NewCommentRequest>,
) -> AppResult<Json<Comment>> {
    let mut conn: DbConnection = get_connection(&state.pool).await?;

    let new_comment = NewComment {
        post_id: payload.post_id,
        user_id: user.id,
        username: user.username,
        comment: payload.comment,
    };

    let saved_comment = diesel::insert_into(comments::table)
        .values(&new_comment)
        .get_result::<Comment>(&mut conn)
        .map_err(DbError::from)?;

    Ok(Json(saved_comment))
}
