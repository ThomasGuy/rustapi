use axum::{
    // extract::{Path, Query},
    routing::{delete, get, patch, post, put},
    Router,
};
// use serde::Deserialize;

use crate::{
    db::DbPool,
    handlers::users::{all_users, create_user},
};

pub fn user_routes(pool: DbPool) -> Router {
    Router::new()
        .route("/api/user", get(all_users))
        .route("/api/user", post(create_user))
        .with_state(pool)
}
