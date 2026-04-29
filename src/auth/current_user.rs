use std::sync::Arc;

use super::{Claims, TokenKeys};
use crate::db::{get_connection, DbConnection, DbPool};
use crate::schema::users;
use crate::utils::AppError;
use axum::{
    extract::{FromRef, FromRequestParts},
    http::request::Parts,
};
use diesel::prelude::*;
use uuid::Uuid;

use crate::models::users::User;

pub struct CurrentUser(pub User);

impl<S> FromRequestParts<S> for CurrentUser
where
    DbPool: FromRef<S>,
    Arc<TokenKeys>: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        // 1. Get claims from the existing extractor
        let claims = Claims::from_request_parts(parts, state).await?;

        // Parse String back to UUID
        let user_uuid = Uuid::parse_str(&claims.sub)
            .map_err(|_| AppError::Auth("Invalid UUID in token".into()))?;

        // 2. Get DB pool
        let pool: DbPool = DbPool::from_ref(state);
        let mut conn: DbConnection = get_connection(&pool).await?;

        // 3. Fetch user from Diesel
        let user = users::table
            // Diesel handles the Uuid type comparison
            .filter(users::id.eq(user_uuid))
            .first::<User>(&mut conn)
            .map_err(|_| AppError::Auth("User not found".into()))?;

        Ok(CurrentUser(user))
    }
}
