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
use tracing::{error, info, warn};
use uuid::Uuid;

use super::UserSummary;
use crate::schema::{likes, posts};
use crate::{auth::CurrentUser, utils::delete_asset_from_sanity};
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
    #[serde(alias = "_ref", rename(serialize = "_ref", deserialize = "reference"))]
    pub reference: String,
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
    pub id: Uuid,
    pub user_id: Uuid,
    pub caption: Option<String>,
    pub user: UserSummary,
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
        user: UserSummary {
            username: post.username,
            avatar_url: user.avatar_url,
        },
        sanity_image: post.sanity_image,
        created_at: post.created_at,
        view_count: post.view_count,
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

    let post_lookup = posts::table
        .filter(posts::id.eq(post_id))
        .filter(posts::user_id.eq(user.id)) // ◄ Ownership validation happens first!
        .select(Post::as_select())
        .first::<Post>(&mut conn)
        .optional() // Wrap in an optional to handle missing records cleanly
        .map_err(DbError::from)?;

    let post = match post_lookup {
        Some(data) => data,
        None => {
            error!(user_id = %user.id, post_id = %post_id, "Post not found or unauthorized");
            return Err(AppError::Forbidden(
                "Unauthorized -- not your post or post does not exist --".into(),
            ));
        }
    };

    let target_asset_id = post.sanity_image.asset.reference;

    // 🚀 TYPE-SAFE & CLEAN: Uses a single sql fragment inside your standard query builder
    let duplicate_refs: i64 = posts::table
        .filter(
            diesel::dsl::sql::<diesel::sql_types::Bool>("(sanity_image->'asset'->>'_ref') = ")
                .bind::<diesel::sql_types::Text, _>(&target_asset_id),
        )
        .filter(posts::id.ne(post_id)) // ◄ Diesel native handler handles the second parameter perfectly!
        .count()
        .get_result::<i64>(&mut conn)
        .unwrap_or(0);

    // 3. Conditional Asset Eviction Execution Path
    if duplicate_refs == 0 {
        info!(id = %target_asset_id, "No duplicate rows found. Initialising remote CDN garbage collection...");
        delete_asset_from_sanity(&state, &target_asset_id).await?;
    } else {
        warn!(
            id = %target_asset_id,
            count = duplicate_refs,
            "Skipping Sanity CDN deletion. Asset is still shared by other post records."
        );
    }

    diesel::delete(posts::table.filter(posts::id.eq(post_id))) // Ownership check)
        .execute(&mut conn)
        .map_err(DbError::from)?;

    info!(user_id=%user.id, "Post deleted");

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
