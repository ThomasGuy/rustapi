use axum::{extract::Multipart, Json};
use serde::Serialize;

// use tokio::fs;
use uuid::Uuid;

use crate::utils::{AppError, AppResult, DbError};

#[derive(Serialize)]
pub struct ImgPath {
    pub filename: String,
}

pub async fn upload_image(mut multipart: Multipart) -> AppResult<Json<ImgPath>> {
    let mut final_filename = None;

    if let Some(field) = multipart.next_field().await? {
        let file_name = field.file_name().unwrap_or("image.jpg").to_string();
        let data = field.bytes().await?;
        let file_size = data.len();

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
        tokio::fs::write(&save_path, data)
            .await
            .map_err(DbError::from)?;

        tracing::info!(filename = %unique_name, bytes = file_size, "Image uploaded successfully");
        final_filename = Some(unique_name);
    }

    let name = final_filename.ok_or_else(|| AppError::Internal("No image file provided".into()))?;
    Ok(Json(ImgPath { filename: name }))
}
