use axum::{
    extract::{Multipart, State},
    http::StatusCode,
    Json,
};
use diesel::RunQueryDsl;
use serde::Serialize;

use std::path::Path;
use tokio::fs;
use uuid::Uuid;

use crate::schema::posts::dsl::*;
use crate::{
    db::{get_connection, DbConnection, DbPool},
    models::posts::{NewPost, Post},
    utils::db_error::{AppResult, DbError},
};

pub async fn create_posts(
    State(pool): State<DbPool>,
    Json(payload): Json<NewPost>,
) -> AppResult<(StatusCode, Json<Post>)> {
    let mut conn: DbConnection = get_connection(&pool).await?;

    let post = diesel::insert_into(posts)
        .values(&payload)
        .get_result::<Post>(&mut conn)
        .map_err(DbError::from)?; // Converts to DbError, then ? converts to AppError

    Ok((StatusCode::CREATED, Json(post)))
}

#[derive(Serialize)]
pub struct ImgPath {
    pub filename: String,
}

pub async fn upload_image(
    State(mut _pool): State<DbPool>,
    mut multipart: Multipart,
) -> AppResult<Json<ImgPath>> {
    let mut final_filename = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(DbError::MultipartError)?
    {
        let file_name = field.file_name().unwrap_or("image.jpg").to_string();
        let data = field.bytes().await.map_err(DbError::MultipartError)?;

        // 1. Generate unique filename
        let extension = Path::new(&file_name)
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("jpg");

        let unique_name = format!("{}.{}", Uuid::new_v4(), extension);
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
