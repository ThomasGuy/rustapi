use std::sync::Arc;

use crate::auth::claims::TokenKeys;
use crate::db::DbPool;
use crate::{config::AppConfig, utils::app_error::AppError};
use axum::extract::FromRef;
use axum_macros::FromRequest;
use serde::{Deserialize, Serialize};

pub type AppResult<T> = Result<T, AppError>;
// 1. Define your shared state
#[derive(Clone, FromRef)]
pub struct AppState {
    pub pool: DbPool,
    pub config: Arc<AppConfig>,
    pub(crate) public_keys: Arc<TokenKeys>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ImageUrlType {
    Relative,
    Absolute,
}

impl ImageUrlType {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Relative => "relative",
            Self::Absolute => "absolute",
        }
    }
}

impl From<ImageUrlType> for String {
    fn from(t: ImageUrlType) -> Self {
        t.as_str().to_string()
    }
}

// custom extractor
#[derive(FromRequest)]
#[from_request(via(axum::Json), rejection(AppError))]
pub struct AppJson<T>(pub T);
