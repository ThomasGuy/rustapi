// use axum::http::StatusCode;
use axum::{extract::State, Json};
use diesel::prelude::*;
use hex;
use jsonwebtoken::{decode, Validation};
use serde::Deserialize;
use sha2::{Digest, Sha256}; // Add 'sha2' crate to hash tokens before storing
use uuid::Uuid;

use crate::auth::{encode_token, Claims, TokenType};
use crate::db::{get_connection, DbConnection};
use crate::models::users::User;
use crate::schema::{refresh_tokens, users};
use crate::utils::{AppError, AppJson, AppResult, AppState, DbError};

use super::AuthResponse;

#[derive(Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

// POST /user/refresh
pub async fn refresh_handler(
    State(state): State<AppState>,
    AppJson(payload): AppJson<RefreshRequest>,
) -> AppResult<Json<AuthResponse>> {
    let mut conn: DbConnection = get_connection(&state.pool).await?;
    let keys = &state.public_keys;

    // 1. Decode and validate the provided refresh token
    let token_data = decode::<Claims>(
        &payload.refresh_token,
        &keys.decoding_key,
        &Validation::default(),
    )
    .map_err(|_| AppError::Auth("Invalid refresh token".into()))?;

    // 2. Hash the incoming token to look it up in DB
    let hash_bytes = Sha256::digest(payload.refresh_token.as_bytes());
    let token_hash = hex::encode(hash_bytes);

    // 3. Find and Delete the token (Atomic Rotation)
    // If delete returns 0 rows, the token was either fake or already used (potential theft!)
    let deleted_count = diesel::delete(refresh_tokens::table)
        .filter(refresh_tokens::token_hash.eq(&token_hash))
        .execute(&mut conn)
        .map_err(|_| AppError::Internal("Database error".into()))?;

    if deleted_count == 0 {
        // Detect potential Reuse: If token is valid JWT but not in DB,
        // someone might be trying to reuse a rotated token.
        return Err(AppError::Auth(
            "Token has already been used or revoked".into(),
        ));
    }

    // 2. Ensure it actually IS a refresh token
    if let TokenType::Access = token_data.claims.token_type {
        return Err(AppError::Auth("Invalid token type".into()));
    }

    // 3. Parse user_id from sub
    let user_id = Uuid::parse_str(&token_data.claims.sub)
        .map_err(|_| AppError::Internal("Invalid user ID in token".into()))?;

    // 4. (Optional) Check DB if user still exists/is active
    let user = users::table
        .find(user_id)
        .first::<User>(&mut conn)
        .map_err(|_| AppError::Auth("User no longer exists".into()))?;

    // 5. Issue new tokens
    let new_access = encode_token(user.id, keys, 15, TokenType::Access, &state)?;
    let new_refresh_raw = encode_token(user.id, keys, 10080, TokenType::Refresh, &state)?;

    // 6. Hash and Store the NEW refresh token
    let hash_bytes = Sha256::digest(new_refresh_raw.as_bytes());
    let new_hash = hex::encode(hash_bytes);
    diesel::insert_into(refresh_tokens::table)
        .values((
            refresh_tokens::user_id.eq(user.id),
            refresh_tokens::token_hash.eq(new_hash),
            refresh_tokens::expires_at.eq(chrono::Utc::now() + chrono::Duration::days(7)),
        ))
        .execute(&mut conn)
        .map_err(DbError::from)?;

    Ok(Json(AuthResponse {
        access_token: new_access,
        refresh_token: new_refresh_raw,
        token_type: "Bearer".to_string(),
        user,
    }))
}
