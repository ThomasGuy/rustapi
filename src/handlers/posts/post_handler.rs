use std::io::Write;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use diesel::deserialize::{self, FromSql};
use diesel::pg::Pg;
use diesel::prelude::*;
use diesel::serialize::{self, IsNull, Output, ToSql};
use diesel::sql_types::Jsonb;
use diesel::{backend::Backend, RunQueryDsl};
use serde::{Deserialize, Serialize};
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

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct SanityAssetRef {
    // 🌟 Bridges incoming JSON "_ref" (or database "reference") to this Rust property
    #[serde(alias = "_ref", rename(serialize = "_ref", deserialize = "reference"))]
    pub reference: String,

    // 🌟 Bridges incoming JSON "_type" (or database "asset_type") to this Rust property
    #[serde(
        alias = "_type",
        rename(serialize = "_type", deserialize = "asset_type")
    )]
    pub asset_type: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SanityHotspot {
    pub x: f64,
    pub y: f64,
    pub height: f64,
    pub width: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SanityCrop {
    pub top: f64,
    pub bottom: f64,
    pub left: f64,
    pub right: f64,
}

#[derive(
    Serialize, Deserialize, Debug, Clone, PartialEq, diesel::FromSqlRow, diesel::AsExpression,
)]
#[diesel(sql_type = diesel::sql_types::Jsonb)]
pub struct SanityImage {
    pub asset: SanityAssetRef,
    pub hotspot: Option<SanityHotspot>,
    pub crop: Option<SanityCrop>,
}

impl FromSql<Jsonb, Pg> for SanityImage {
    fn from_sql(bytes: <Pg as Backend>::RawValue<'_>) -> deserialize::Result<Self> {
        let value = <serde_json::Value as FromSql<Jsonb, Pg>>::from_sql(bytes)?;
        let image: SanityImage = serde_json::from_value(value)?;
        Ok(image)
    }
}

impl ToSql<Jsonb, Pg> for SanityImage {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        // 1. Write the mandatory Postgres JSONB version header (currently 1)
        out.write_all(&[1])?;
        // 2. Stream your struct bytes directly into the output network buffer
        serde_json::to_writer(out, self)?;
        Ok(IsNull::No)
    }
}

// 🌟 Updated: Structural transition to match the full payload contract
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageRequest {
    pub caption: Option<String>,
    pub sanity_image: SanityImage, // Accept full layout constraints from UI
}

// 🌟 Updated: Echo the full structure back out to the frontend application
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatePostResponse {
    pub id: uuid::Uuid,
    pub user_id: uuid::Uuid,
    pub caption: Option<String>,
    pub username: String,
    pub sanity_image: SanityImage, // Becomes "sanityImage" object array in TypeScript
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
        sanity_image: payload.sanity_image,
    };

    let post = diesel::insert_into(posts::table)
        .values(&new_post)
        .returning(Post::as_select())
        .get_result::<Post>(&mut conn)
        .map_err(DbError::from)?; // Converts to DbError, then ? converts to AppError

    tracing::info!(user_id=%post.user_id, user_name=%post.username, post_id=%post.id, "new post");

    // 2. Map the database model into your clean web response struct
    let response_payload = CreatePostResponse {
        id: post.id,
        user_id: post.user_id,
        caption: post.caption,
        username: post.username,
        sanity_image: post.sanity_image,
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
