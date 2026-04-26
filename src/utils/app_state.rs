use std::sync::{Arc, RwLock};

use crate::config::AppConfig;
use crate::db::DbPool;
use axum::extract::FromRef;

// 1. Define your shared state
#[derive(Clone, FromRef)]
pub struct AppState {
    pub pool: DbPool,
    pub(crate) config: Arc<AppConfig>,
    pub(crate) public_keys: Arc<RwLock<TokenKeys>>,
}

pub struct TokenKeys {
    pub(crate) token_key: String,
}
