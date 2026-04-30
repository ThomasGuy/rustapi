use std::sync::Arc;

use axum::extract::FromRef;
use axum_macros::FromRequest;

use super::super::{AppConfig, AppError};
use crate::auth::TokenKeys;
use crate::db::DbPool;

// 1. Define your shared state
#[derive(Clone, FromRef)]
pub struct AppState {
    pub pool: DbPool,
    pub config: Arc<AppConfig>,
    pub(crate) public_keys: Arc<TokenKeys>,
}

// custom extractor
#[derive(FromRequest)]
#[from_request(via(axum::Json), rejection(AppError))]
pub struct AppJson<T>(pub T);

pub type AppResult<T> = Result<T, AppError>;
