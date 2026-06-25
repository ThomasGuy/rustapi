use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{extract::State, Json};
use diesel::dsl::now;
use diesel::prelude::*;
use hex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tower_cookies::{
    cookie::{time::Duration, SameSite},
    Cookie, Cookies,
};

use crate::auth::{encode_token, CurrentUser, TokenType};
use crate::db::{get_connection, DbConnection};
use crate::models::users::{UpdateUserPayload, User};
use crate::schema::{refresh_tokens, users};
use crate::utils::{verify_password, AppError, AppJson, AppResult, AppState, DbError, Environment};

#[derive(Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct AuthResponse {
    #[serde(rename = "authToken")]
    pub access_token: String,
    #[serde(rename = "authTokenType")]
    pub token_type: String,
    pub user: User,
}

// POST /user/login
#[tracing::instrument(skip(state, cookies, payload), fields(user.username = %payload.username))]
pub async fn login(
    State(state): State<AppState>,
    cookies: Cookies,
    AppJson(payload): AppJson<LoginRequest>,
) -> AppResult<Json<AuthResponse>> {
    let mut conn: DbConnection = get_connection(&state.pool).await?;
    let secure_flag = state.config.app_env.requires_secure_cookies();
    let samesite_policy = if state.config.app_env == Environment::Local {
        SameSite::Lax // Required so the cookie attaches to fresh cross-port tab loads
    } else {
        SameSite::Strict
    };

    // 1. Find user
    let user = users::table
        .filter(users::username.eq(&payload.username))
        .first::<User>(&mut conn)
        .map_err(|_| AppError::Auth("User not found".into()))?;

    // 2. Check password
    if !verify_password(&payload.password, &user.password_hash)? {
        return Err(AppError::Auth("Invalid password".into()));
    }

    // 2.a update user last_login_at
    diesel::update(users::table.filter(users::id.eq(user.id)))
        .set(users::last_login_at.eq(now))
        .execute(&mut conn)
        .expect("Error updating last login");

    // 3. Generate tokens (Access & Refresh)
    let keys = &state.public_keys;
    let access_token = encode_token(user.id, keys, 15, TokenType::Access, &state)?; // 15 mins
    let refresh_token_raw = encode_token(user.id, keys, 10080, TokenType::Refresh, &state)?; // 7 days

    let hash_bytes = Sha256::digest(refresh_token_raw.as_bytes());
    let refresh_hash = hex::encode(hash_bytes);

    diesel::insert_into(refresh_tokens::table)
        .values((
            refresh_tokens::user_id.eq(user.id),
            refresh_tokens::token_hash.eq(refresh_hash),
            refresh_tokens::expires_at.eq(chrono::Utc::now() + chrono::Duration::days(7)),
        ))
        .execute(&mut conn)
        .map_err(|e| AppError::Internal(format!("Failed to store session: {}", e)))?;

    // 4. Build a standard string cookie using tower-cookies' plain API
    let mut cookie = Cookie::new("refresh_token", refresh_token_raw);
    cookie.set_path("/");
    cookie.set_http_only(true);
    cookie.set_secure(secure_flag); // Must be true on production VPS
    cookie.set_same_site(samesite_policy);
    cookie.set_max_age(Some(Duration::days(7))); // 7 days matches refresh_token

    tracing::info!(user_id=%user.id, user_name=%user.username, "login success");

    // 5. Inject into cookie manager state in-place (No tuples returned!)
    cookies.add(cookie);

    Ok(Json(AuthResponse {
        access_token,
        token_type: "Bearer".to_string(),
        user,
    }))
}

// POST /user.logout
pub async fn logout(
    State(state): State<AppState>,
    cookies: Cookies,
) -> AppResult<impl IntoResponse> {
    let mut conn: DbConnection = get_connection(&state.pool).await?;

    // 1. If a cookie exists, extract and delete it from the Postgres database
    if let Some(cookie) = cookies.get("refresh_token") {
        let token_raw = cookie.value();

        // Hash it exactly how you do during login/refresh lookup
        let hash_bytes = Sha256::digest(token_raw.as_bytes());
        let token_hash = hex::encode(hash_bytes);

        // Delete from Diesel schema to invalidate on the server side
        let _ = diesel::delete(refresh_tokens::table)
            .filter(refresh_tokens::token_hash.eq(&token_hash))
            .execute(&mut conn);
    }

    // 2. Build an identical cookie structured to overwrite and immediately expire
    let secure_flag = state.config.app_env.requires_secure_cookies();
    let samesite_policy = if state.config.app_env == Environment::Local {
        SameSite::Lax
    } else {
        SameSite::Strict
    };

    let mut deletion_cookie = Cookie::new("refresh_token", "");
    deletion_cookie.set_path("/");
    deletion_cookie.set_http_only(true);
    deletion_cookie.set_secure(secure_flag);
    deletion_cookie.set_same_site(samesite_policy);
    deletion_cookie.set_max_age(Some(Duration::ZERO)); // Crucial: Evicts the cookie instantly

    // 3. Push to the cookie manager layer
    cookies.add(deletion_cookie);

    Ok(StatusCode::NO_CONTENT)
}

// PATCH /user/update
#[tracing::instrument(skip(state, current_user, payload), fields(user.id = %current_user.id))]
pub async fn update_profile(
    State(state): State<AppState>,
    CurrentUser(current_user): CurrentUser,
    AppJson(mut payload): AppJson<UpdateUserPayload>,
) -> AppResult<Json<UpdateUserPayload>> {
    let current_time = chrono::Utc::now().naive_utc();
    payload.updated_at = Some(current_time);
    let sanity = &state.config.sanity_config;
    let mut conn: DbConnection = get_connection(&state.pool).await?;

    let old_asset_ids: Vec<Option<String>> = users::table
        .filter(users::id.eq(current_user.id))
        .select(users::avatar_url)
        .load::<Option<String>>(&mut conn)
        .map_err(DbError::from)?;

    let old_asset_id = old_asset_ids.first().cloned().flatten();

    // 2. Wrap the blocking Diesel update query inside a scoped code block
    {
        let mut conn: DbConnection = get_connection(&state.pool).await?;

        diesel::update(users::table.filter(users::id.eq(current_user.id)))
            .set(&payload)
            .execute(&mut conn) // Light database operation, returns row count instead of records
            .map_err(DbError::DatabaseError)?;
    };

    tracing::info!(user_id = %current_user.id, "Profile updated successfully");

    // 🚀 DIAGNOSTIC IMPROVEMENT: Trace out what values are actually sitting here
    tracing::info!(
        "Purge Check -> Old ID in DB: {:?}, New Payload ID: {:?}",
        old_asset_id,
        payload.avatar_url
    );

    // 🚀 Hook up Sanity background task if the asset changed and an old one exists
    if let Some(old_id) = old_asset_id {
        // Only trigger if the payload actually changed the avatar string
        if Some(&old_id) != payload.avatar_url.as_ref() {
            let project_id = sanity.project_id.clone();
            let dataset = sanity.dataset.clone();
            let token = sanity.write_token.clone();

            tokio::spawn(async move {
                let client = reqwest::Client::new();
                let url = format!(
                    "https://{}.api.sanity.io/v2026-06-25/data/mutate/{}",
                    project_id, dataset
                );

                let mutation_payload = serde_json::json!({
                    "mutations": [{ "delete": { "id": old_id } }]
                });

                match client
                    .post(&url)
                    .header("Authorization", format!("Bearer {}", token))
                    .json(&mutation_payload)
                    .send()
                    .await
                {
                    Ok(res) => {
                        let status = res.status();
                        // Read body text to see what Sanity is complaining about if it breaks
                        let body_text = res.text().await.unwrap_or_default();

                        if status.is_success() {
                            tracing::info!("Successfully purged orphaned avatar from Sanity");
                        } else {
                            tracing::warn!(
                                "Sanity rejected asset delete with status: {} - Details: {}",
                                status,
                                body_text
                            );
                        }
                    }
                    Err(e) => {
                        tracing::error!(
                            "Failed sending deletion mutation request to Sanity: {:?}",
                            e
                        );
                    }
                }
            });
        } else {
            tracing::info!("Purge skipped: Old ID matches incoming Payload ID exactly.");
        }
    } else {
        tracing::info!("Purge skipped: No old avatar ID was found in the database profile row.");
    }

    Ok(Json(payload))
}
