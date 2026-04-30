use crate::auth::{encode_token, CurrentUser, TokenType};
use crate::db::{get_connection, DbConnection};
use crate::models::users::User;
use crate::schema::{refresh_tokens, users};
use crate::utils::{verify_password, AppError, AppJson, AppResult, AppState};
use axum::http::StatusCode;
use axum::{extract::State, Json};
use diesel::prelude::*;
use hex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

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
    pub refresh_token: String,
    pub user: User,
}

#[tracing::instrument(skip(state, payload), fields(user.username = %payload.username))]
pub async fn login(
    State(state): State<AppState>,
    AppJson(payload): AppJson<LoginRequest>,
) -> AppResult<Json<AuthResponse>> {
    let mut conn: DbConnection = get_connection(&state.pool).await?;

    // 1. Find user
    let user = users::table
        .filter(users::username.eq(&payload.username))
        .first::<User>(&mut conn)
        .map_err(|_| AppError::Auth("User not found".into()))?;

    // 2. Check password
    if !verify_password(&payload.password, &user.password_hash)? {
        return Err(AppError::Auth("Invalid password".into()));
    }

    // 3. Generate tokens (Access & Refresh)
    let keys = &state.public_keys;
    let access_token = encode_token(user.id, keys, 15, TokenType::Access)?; // 15 mins
    let refresh_token = encode_token(user.id, keys, 10080, TokenType::Refresh)?; // 7 days

    let hash_bytes = Sha256::digest(refresh_token.as_bytes());
    let refresh_hash = hex::encode(hash_bytes);

    diesel::insert_into(refresh_tokens::table)
        .values((
            refresh_tokens::user_id.eq(user.id),
            refresh_tokens::token_hash.eq(refresh_hash),
            refresh_tokens::expires_at.eq(chrono::Utc::now() + chrono::Duration::days(7)),
        ))
        .execute(&mut conn)
        .map_err(|e| AppError::Internal(format!("Failed to store session: {}", e)))?;

    tracing::info!(user_id=%user.id, user_name=%user.username, "login success");
    Ok(Json(AuthResponse {
        access_token,
        token_type: "Bearer".to_string(),
        refresh_token,
        user,
    }))
}

#[derive(Deserialize)]
pub struct LogoutRequest {
    pub refresh_token: String,
}

pub async fn logout(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser, // Optional: ensures user is authenticated via Access Token
    AppJson(payload): AppJson<LogoutRequest>,
) -> AppResult<StatusCode> {
    let mut conn: DbConnection = get_connection(&state.pool).await?;

    // 1. Hash the provided token
    let token_hash = hex::encode(Sha256::digest(payload.refresh_token.as_bytes()));

    // 2. Remove from DB
    let deleted = diesel::delete(refresh_tokens::table)
        .filter(refresh_tokens::token_hash.eq(token_hash))
        .execute(&mut conn)
        .map_err(|_| AppError::Internal("Database error".into()))?;

    if deleted == 0 {
        return Err(AppError::Auth("Session already invalid".into()));
    }
    tracing::info!(user_id=%user.id, user_name=%user.username, "logout success");
    Ok(StatusCode::NO_CONTENT)
}

// pub async fn logout_all_devices(
//     State(state): State<AppState>,
//     claims: Claims, // Extracted from Access Token
// ) -> AppResult<StatusCode> {
//     let mut conn = get_connection(&state.pool).await?;
//     let user_id = Uuid::parse_str(&claims.sub).unwrap();

//     diesel::delete(refresh_tokens::table)
//         .filter(refresh_tokens::user_id.eq(user_id))
//         .execute(&mut conn)
//         .map_err(DbError::from)?;

//     Ok(StatusCode::NO_CONTENT)
// }
