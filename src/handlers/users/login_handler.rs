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

use crate::auth::{encode_token, TokenType};
use crate::db::{get_connection, DbConnection};
use crate::models::users::User;
use crate::schema::{refresh_tokens, users};
use crate::utils::{verify_password, AppError, AppJson, AppResult, AppState, Environment};

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
    cookie.set_max_age(Some(Duration::seconds(7 * 24 * 60 * 60))); // 7 days matches refresh_token

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
