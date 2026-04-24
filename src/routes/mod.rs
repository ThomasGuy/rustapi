pub(crate) mod post_routes;
pub(crate) mod user_routes;

use axum::routing::{delete, get, post, put};
use axum::Router;

use crate::{
    db::DbPool,
    handlers::{
        health,
        users::{all_users, create_user},
    },
};

pub fn create_routes(pool: DbPool) -> Router {
    Router::new()
        .route("/health", get(health::health_check))
        .route("/api/user", get(all_users))
        .route("/api/user", post(create_user))
        .with_state(pool)
}
