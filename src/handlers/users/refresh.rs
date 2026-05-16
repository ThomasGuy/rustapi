use axum::{extract::State, Json};
use diesel::prelude::*;
use hex;
use jsonwebtoken::{decode, Validation};
use sha2::{Digest, Sha256}; // Add 'sha2' crate to hash tokens before storing
use tower_cookies::{
    cookie::{time::Duration, SameSite},
    Cookie, Cookies,
};
use uuid::Uuid;

use crate::auth::{encode_token, Claims, TokenType};
use crate::db::{get_connection, DbConnection};
use crate::models::users::User;
use crate::schema::{refresh_tokens, users};
use crate::utils::{AppError, AppResult, AppState, DbError, Environment};

use super::AuthResponse;

// POST /user/refresh
pub async fn refresh_handler(
    State(state): State<AppState>,
    cookies: Cookies,
) -> AppResult<Json<AuthResponse>> {
    let mut conn: DbConnection = get_connection(&state.pool).await?;
    let keys = &state.public_keys;
    let secure_flag = state.config.app_env.requires_secure_cookies();
    let samesite_policy = if state.config.app_env == Environment::Local {
        SameSite::Lax // Required so the cookie attaches to fresh cross-port tab loads
    } else {
        SameSite::Strict
    };

    // 1. Extract the raw token string directly from the secure browser cookie
    let refresh_token_raw = cookies
        .get("refresh_token")
        .map(|cookie| cookie.value().to_string())
        .ok_or_else(|| AppError::MissingCookie)?;

    // 2. Decode and validate the provided refresh token
    let token_data = decode::<Claims>(
        &refresh_token_raw,
        &keys.decoding_key,
        &Validation::default(),
    )
    .map_err(|_| AppError::Auth("Invalid refresh token".into()))?;

    // 3. Hash the incoming token to look it up in DB
    let hash_bytes = Sha256::digest(refresh_token_raw.as_bytes());
    let token_hash = hex::encode(hash_bytes);

    // 4. Find and Delete the token (Atomic Rotation)
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

    // 5 Ensure it actually IS a refresh token
    if let TokenType::Access = token_data.claims.token_type {
        return Err(AppError::Auth("Invalid token type".into()));
    }

    // 6. Parse user_id from sub
    let user_id = Uuid::parse_str(&token_data.claims.sub)
        .map_err(|_| AppError::Internal("Invalid user ID in token".into()))?;

    // 7. Check DB if user still exists/is active
    let user = users::table
        .find(user_id)
        .first::<User>(&mut conn)
        .map_err(|_| AppError::Auth("User no longer exists".into()))?;

    // 8. Issue new tokens
    let new_access = encode_token(user.id, keys, 15, TokenType::Access, &state)?;
    let new_refresh_raw = encode_token(user.id, keys, 10080, TokenType::Refresh, &state)?;

    // 6. Hash and Store the NEW refresh token
    let hash_bytes = Sha256::digest(new_refresh_raw.as_bytes());
    let new_refresh_hash = hex::encode(hash_bytes);

    diesel::insert_into(refresh_tokens::table)
        .values((
            refresh_tokens::user_id.eq(user.id),
            refresh_tokens::token_hash.eq(new_refresh_hash),
            refresh_tokens::expires_at.eq(chrono::Utc::now() + chrono::Duration::days(7)),
        ))
        .execute(&mut conn)
        .map_err(DbError::from)?;

    // 7. Push the updated cookie string back to the browser engine
    let mut new_cookie = Cookie::new("refresh_token", new_refresh_raw);
    new_cookie.set_path("/");
    new_cookie.set_http_only(true);
    new_cookie.set_secure(secure_flag); // Ensure true for production VPS
    new_cookie.set_same_site(samesite_policy);
    new_cookie.set_max_age(Some(Duration::seconds(7 * 24 * 60 * 60)));

    cookies.add(new_cookie); // Overwrites old browser cookie instantly

    Ok(Json(AuthResponse {
        access_token: new_access,
        token_type: "Bearer".to_string(),
        user,
    }))
}
