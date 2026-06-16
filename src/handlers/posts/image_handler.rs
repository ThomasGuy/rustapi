use axum::{
    extract::{Multipart, State},
    Json,
};
use reqwest::Client;
use serde::Serialize;

use crate::{
    auth::CurrentUser,
    utils::{AppError, AppResult, AppState},
};

#[derive(Serialize)]
pub struct AssetId {
    #[serde(rename = "sanityAssetId")]
    pub sanity_asset_id: String,
}

// POST /post/image
pub async fn upload_image(
    State(state): State<AppState>,
    CurrentUser(_user): CurrentUser,
    mut multipart: Multipart,
) -> AppResult<Json<AssetId>> {
    let mut file_bytes = None; // Holds the processed data block
    let mut content_type = "image/jpeg".to_string();
    let mut file_size = 0;

    // 1. Ingest the file chunks cleanly from the multi-part data payload
    if let Some(field) = multipart.next_field().await? {
        content_type = field.content_type().unwrap_or("image/jpeg").to_string();
        let bytes_buffer = field.bytes().await?;
        file_size = bytes_buffer.len();
        file_bytes = Some(bytes_buffer);
    }

    // Trigger validation error if the field wasn't sent down from React
    let final_upload_body = file_bytes.ok_or_else(|| {
        AppError::Validation("No field named 'file' found in request payload".into())
    })?;

    // 2. Fetch target variables from your global backend app state
    let sanity = &state.config.sanity_config;
    let upload_url = format!(
        "https://{}.api.sanity.io/v2026-05-15/assets/images/{}",
        sanity.project_id, sanity.dataset
    );

    // 3. Directly stream the payload outward to Sanity using the hidden write_token
    let http_client = Client::new();
    let sanity_response = http_client
        .post(&upload_url)
        .header("Authorization", format!("Bearer {}", sanity.write_token))
        .header("Content-Type", content_type)
        .body(final_upload_body) // The stream pipes straight through your VPS memory allocation
        .send()
        .await
        .map_err(AppError::ReqwestError)?; // Automatically catches network / connection hiccups

    if !sanity_response.status().is_success() {
        let error_status = sanity_response.status();
        let error_text = sanity_response
            .text()
            .await
            .unwrap_or_else(|_| "Could not read error body".to_string());

        // Dumps the real error straight to your journalctl/terminal logs!
        tracing::error!(
            status = ?error_status,
            body = %error_text,
            "Sanity API Rejected Outbound Upload Request"
        );

        return Err(AppError::Internal(format!(
            "Sanity API rejected upload with status {}: {}",
            error_status, error_text
        )));
    }
    // 4. Ingest and unpack the returned Sanity Asset ID string reference
    let sanity_json: serde_json::Value = sanity_response
        .json()
        .await
        .map_err(AppError::ReqwestError)?;

    let asset_id = sanity_json["document"]["_id"]
        .as_str()
        .ok_or_else(|| {
            AppError::Internal("Malformed payload structure returned from Sanity".into())
        })?
        .to_string();

    tracing::info!(file_size=%file_size, "image upload to sanity success");

    // 5. Safely pass the asset ID back to your React frontend context
    Ok(Json(AssetId {
        sanity_asset_id: asset_id,
    }))
}
