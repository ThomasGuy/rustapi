use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use tracing::{error, info};

use crate::utils::{AppError, AppResult, AppState};

pub async fn delete_asset_from_sanity(state: &AppState, sanity_asset_id: &str) -> AppResult<()> {
    let sanity = &state.config.sanity_config;

    // 2. Build the secure target endpoint URL specified by Sanity's Asset Management API
    let url = format!(
        "https://{}.api.sanity.io/v2021-06-07/data/mutate/{}?returnIds=true",
        sanity.project_id, sanity.dataset
    );

    // 3. Package the standard Sanity mutation delete payload mutation array
    let mutation_payload = serde_json::json!({
        "mutations": [
            { "delete": { "id": sanity_asset_id } }
        ]
    });

    // 4. Inject your secure write-access token into the headers
    let mut headers = HeaderMap::new();
    let auth_string = format!("Bearer {}", sanity.write_token);
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&auth_string)
            .map_err(|_| AppError::Auth("Authorization failed".into()))?,
    );

    let client = reqwest::Client::new();
    let response = client
        .post(&url)
        .headers(headers)
        .json(&mutation_payload)
        .send()
        .await
        .map_err(|err| AppError::ReqwestError(err))?;

    if response.status().is_success() {
        info!(id = %sanity_asset_id, "Asset deleted successfully");
        Ok(())
    } else {
        // ◄ 🚀 THIS is the exact line that handles your bad tokens and missing files!
        let status = response.status();
        let error_body = response.text().await.unwrap_or_default();

        error!(status = %status, body = %error_body, "Sanity API rejected asset deletion");
        Err(AppError::Sanity(error_body))
    }
}
