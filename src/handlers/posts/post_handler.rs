use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use diesel::prelude::*;
use diesel::RunQueryDsl;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::schema::posts;
use crate::{
    db::{get_connection, DbConnection},
    models::posts::{NewPost, Post},
    utils::{AppJson, AppResult, AppState, DbError},
};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ImageUrlType {
    Relative,
    Absolute,
}

impl ImageUrlType {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Relative => "relative",
            Self::Absolute => "absolute",
        }
    }
}

impl From<ImageUrlType> for String {
    fn from(t: ImageUrlType) -> Self {
        t.as_str().to_string()
    }
}

#[derive(Debug, Deserialize)]
pub struct ImageRequest {
    caption: Option<String>,
    image_url: String,
    image_url_type: ImageUrlType,
}

pub async fn create_posts(
    CurrentUser(user): CurrentUser,
    State(state): State<AppState>,
    AppJson(payload): AppJson<ImageRequest>,
) -> AppResult<(StatusCode, Json<Post>)> {
    let mut conn: DbConnection = get_connection(&state.pool).await?;

    let new_post = NewPost {
        user_id: user.id,
        caption: payload.caption,
        username: user.username,
        image_url: payload.image_url,
        image_url_type: payload.image_url_type.into(),
    };

    let post = diesel::insert_into(posts::table)
        .values(&new_post)
        .get_result::<Post>(&mut conn)
        .map_err(DbError::from)?; // Converts to DbError, then ? converts to AppError

    tracing::info!(user_id=%post.user_id, user_name=%post.username, post_id=%post.id, "new post");
    Ok((StatusCode::CREATED, Json(post)))
}

pub async fn delete_post(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(post_id): Path<Uuid>, // The ID of the post to delete
) -> AppResult<StatusCode> {
    let mut conn: DbConnection = get_connection(&state.pool).await?;

    // Only delete if BOTH the post_id and user_id match
    let count = diesel::delete(
        posts::table
            .filter(posts::id.eq(post_id))
            .filter(posts::user_id.eq(user.id)), // Ownership check
    )
    .execute(&mut conn)
    .map_err(DbError::from)?;

    if count == 0 {
        tracing::error!(user_id=%user.id, "No post to delete");
        // If no rows were deleted, either the post doesn't exist
        // or the current user doesn't own it.
        // let db_err = DbError::NotFound("Post not found or unauthorized".into());
        // return Err(AppError::Db(db_err));
        return Err(DbError::NotFound("Post not found or unauthorized".into()).into());
    }
    tracing::info!(user_id=%user.id, "Post deleted");
    Ok(StatusCode::NO_CONTENT)
}
