use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use axum::{
    extract::{FromRef, FromRequestParts},
    http::request::Parts,
};
use axum_extra::{
    headers::{authorization::Bearer, Authorization},
    TypedHeader,
};
use diesel::prelude::*;
use jsonwebtoken::{decode, encode, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::DbConnection;
use crate::schema::users;
use crate::utils::{AppError, AppState, DbError};

pub struct TokenKeys {
    pub encoding_key: jsonwebtoken::EncodingKey,
    pub decoding_key: jsonwebtoken::DecodingKey,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum TokenType {
    Access,
    Refresh,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String, // User ID
    pub exp: usize,  // Expiration time
    pub token_type: TokenType,
    pub is_admin: bool,
}

impl<S> FromRequestParts<S> for Claims
where
    Arc<TokenKeys>: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        // 1. Extract the Authorization header (Bearer token)
        let TypedHeader(Authorization(bearer)) =
            TypedHeader::<Authorization<Bearer>>::from_request_parts(parts, state)
                .await
                .map_err(|_| AppError::Auth("Missing Authorization header".into()))?;

        // 2. Get the Keys from State
        let keys = Arc::<TokenKeys>::from_ref(state);

        // 4. Decode using your keys logic
        let token_data =
            decode::<Claims>(bearer.token(), &keys.decoding_key, &Validation::default())
                .map_err(|e| AppError::Auth(format!("JWT Error: {}", e)))?;

        if let TokenType::Refresh = token_data.claims.token_type {
            return Err(AppError::Auth(
                "Cannot use refresh token for authorization".into(),
            ));
        }

        Ok(token_data.claims)
    }
}

pub fn encode_token(
    user_id: Uuid,
    keys: &TokenKeys,
    minutes: i64,
    token_type: TokenType,
    state: &AppState,
) -> Result<String, AppError> {
    let mut conn: DbConnection = state.pool.get().map_err(DbError::from)?;

    let exp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| AppError::Internal("Time went backwards".into()))?
        .as_secs() as i64
        + (minutes * 60);

    // Look up the is_admin status for this specific user
    let admin_status: bool = users::table
        .filter(users::id.eq(user_id))
        .select(users::is_admin)
        .first(&mut conn)
        .map_err(DbError::from)?;

    let claims = Claims {
        sub: user_id.to_string(),
        exp: exp as usize,
        token_type,
        is_admin: admin_status,
    };

    // Use &keys.encoding_key (ensure you have EncodingKey stored in your struct)
    encode(&Header::default(), &claims, &keys.encoding_key)
        .map_err(|e| AppError::Internal(format!("JWT error: {}", e)))
}
