use axum::{
    extract::{Multipart, Path, State},
    http::StatusCode,
    Json,
};
use diesel::prelude::*;
use diesel::RunQueryDsl;
use serde::{Deserialize, Serialize};

use tokio::fs;
use uuid::Uuid;

use crate::auth::current_user::CurrentUser;
use crate::schema::posts;
use crate::{
    db::{get_connection, DbConnection, DbPool},
    models::posts::{NewPost, Post},
    utils::{
        app_state::{AppJson, AppResult, ImageUrlType},
        db_error::DbError,
    },
};

#[derive(Debug, Deserialize)]
pub struct ImageRequest {
    caption: Option<String>,
    image_url: String,
    image_url_type: ImageUrlType,
}

pub async fn create_posts(
    CurrentUser(user): CurrentUser,
    State(pool): State<DbPool>,
    AppJson(payload): AppJson<ImageRequest>,
) -> AppResult<(StatusCode, Json<Post>)> {
    let new_post = NewPost {
        user_id: user.id,
        caption: payload.caption,
        image_url: payload.image_url,
        image_url_type: payload.image_url_type.into(),
    };

    let mut conn: DbConnection = get_connection(&pool).await?;

    let post = diesel::insert_into(posts::table)
        .values(&new_post)
        .get_result::<Post>(&mut conn)
        .map_err(DbError::from)?; // Converts to DbError, then ? converts to AppError

    Ok((StatusCode::CREATED, Json(post)))
}

#[derive(Serialize)]
pub struct ImgPath {
    pub filename: String,
}

pub async fn upload_image(mut multipart: Multipart) -> AppResult<Json<ImgPath>> {
    let mut final_filename = None;

    while let Some(field) = multipart.next_field().await? {
        let file_name = field.file_name().unwrap_or("image.jpg").to_string();
        let data = field.bytes().await?;

        println!("Length of `{}` is {} bytes", file_name, data.len());

        // 1. Generate unique filename
        let stem = std::path::Path::new(&file_name)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("image");

        let ext = std::path::Path::new(&file_name)
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("jpg");

        let unique_name = format!("{}_{}.{}", Uuid::new_v4(), stem, ext);
        let save_path = format!("./images/{}", unique_name);

        // 2. Save file asynchronously
        fs::write(&save_path, data).await.map_err(DbError::from)?;
        final_filename = Some(unique_name);
    }

    Ok(final_filename
        .map(|name| Json(ImgPath { filename: name }))
        // to satisfy the option, we have to supply a value for the fn - hence all this
        .ok_or_else(|| {
            DbError::UploadError(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "No image file found",
            ))
        })?)
}

pub async fn get_my_posts(
    State(pool): State<DbPool>,
    CurrentUser(user): CurrentUser,
) -> AppResult<Json<Vec<Post>>> {
    let mut conn: DbConnection = get_connection(&pool).await?;

    let my_posts = posts::table
        .filter(posts::user_id.eq(user.id))
        .load::<Post>(&mut conn)
        .map_err(DbError::from)?;

    Ok(Json(my_posts))
}

pub async fn delete_post(
    State(pool): State<DbPool>,
    CurrentUser(user): CurrentUser,
    Path(post_id): Path<Uuid>, // The ID of the post to delete
) -> AppResult<StatusCode> {
    let mut conn: DbConnection = get_connection(&pool).await?;

    // Only delete if BOTH the post_id and user_id match
    let count = diesel::delete(
        posts::table
            .filter(posts::id.eq(post_id))
            .filter(posts::user_id.eq(user.id)), // Ownership check
    )
    .execute(&mut conn)
    .map_err(DbError::from)?;

    if count == 0 {
        // If no rows were deleted, either the post doesn't exist
        // or the current user doesn't own it.
        // let db_err = DbError::NotFound("Post not found or unauthorized".into());
        // return Err(AppError::Db(db_err));
        return Err(DbError::NotFound("Post not found or unauthorized".into()).into());
    }

    Ok(StatusCode::NO_CONTENT)
}
