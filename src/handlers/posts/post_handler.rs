use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use diesel::prelude::*;
use diesel::RunQueryDsl;
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::schema::{likes, posts};
use crate::{
    db::{get_connection, DbConnection},
    models::{
        likes::Like,
        posts::{NewPost, Post},
    },
    utils::{AppError, AppJson, AppResult, AppState, DbError},
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageRequest {
    caption: Option<String>,
    sanity_asset_id: String,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatePostResponse {
    pub id: uuid::Uuid,
    pub user_id: Uuid,
    pub caption: Option<String>,
    pub username: String,
    pub sanity_asset_id: String, // Becomes "sanityAssetId" in TypeScript
    pub view_count: i32,
    pub created_at: chrono::NaiveDateTime,
}

// POST /post/create
pub async fn create_posts(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    AppJson(payload): AppJson<ImageRequest>,
) -> AppResult<(StatusCode, Json<CreatePostResponse>)> {
    let mut conn: DbConnection = get_connection(&state.pool).await?;

    let new_post = NewPost {
        user_id: user.id,
        caption: payload.caption,
        username: user.username,
        sanity_asset_id: payload.sanity_asset_id,
    };

    let post = diesel::insert_into(posts::table)
        .values(&new_post)
        .get_result::<Post>(&mut conn)
        .map_err(DbError::from)?; // Converts to DbError, then ? converts to AppError

    tracing::info!(user_id=%post.user_id, user_name=%post.username, post_id=%post.id, "new post");

    // 2. Map the database model into your clean web response struct
    let response_payload = CreatePostResponse {
        id: post.id,
        user_id: post.user_id,
        caption: post.caption,
        username: post.username,
        sanity_asset_id: post.sanity_asset_id,
        view_count: post.view_count,
        created_at: post.created_at,
    };

    Ok((StatusCode::CREATED, Json(response_payload)))
}

// DELETE /post/delete/{id}
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
        tracing::error!(user_id=%user.id, "Post nor found or unauthorized");
        // If no rows were deleted, either the post doesn't exist
        // or the current user doesn't own it.
        // let db_err = DbError::NotFound("Post not found or unauthorized".into());
        // return Err(AppError::Db(db_err));
        return Err(AppError::Forbidden(
            "Unauthorized -- not your post --".into(),
        ));
    }
    tracing::info!(user_id=%user.id, "Post deleted");
    Ok(StatusCode::NO_CONTENT)
}

// POST /post/like/{id}
pub async fn toggle_like(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(post_id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    let mut conn: DbConnection = get_connection(&state.pool).await?;

    // 1. Check if the like exists
    let existing_like = likes::table
        .filter(likes::user_id.eq(user.id))
        .filter(likes::post_id.eq(post_id))
        .first::<Like>(&mut conn)
        .optional() // Returns Ok(None) if not found instead of an error
        .map_err(DbError::from)?;

    match existing_like {
        Some(_) => {
            // 2. UNLIKE: It exists, so remove it
            diesel::delete(likes::table)
                .filter(likes::user_id.eq(user.id))
                .filter(likes::post_id.eq(post_id))
                .execute(&mut conn)
                .map_err(DbError::from)?;

            Ok(Json(
                serde_json::json!({ "status": "unliked", "post_id": post_id }),
            ))
        }
        None => {
            // 3. LIKE: It doesn't exist, so add it
            diesel::insert_into(likes::table)
                .values((likes::user_id.eq(user.id), likes::post_id.eq(post_id)))
                .execute(&mut conn)
                .map_err(DbError::from)?;

            Ok(Json(
                serde_json::json!({ "status": "liked", "post_id": post_id }),
            ))
        }
    }
}
