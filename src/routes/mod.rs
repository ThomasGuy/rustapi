// pub(crate) mod post_routes;
// pub(crate) mod user_routes;

use axum::{
    // extract::State,
    routing::{get, post},
    Router,
};

use crate::{
    // db::DbPool,
    handlers::{
        health,
        users::{all_users, create_user, get_config},
    },
    utils::app_state::AppState,
};

pub fn create_routes(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health::health_check))
        .route("/api/user", get(all_users))
        .route("/api/user", post(create_user))
        .route("/config", get(get_config))
        .with_state(state)
}
