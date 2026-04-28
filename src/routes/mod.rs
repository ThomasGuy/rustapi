use axum::{
    extract::DefaultBodyLimit,
    routing::{delete, get, post},
    Router,
};
use tower_http::{limit::RequestBodyLimitLayer, services::ServeDir};

use crate::{
    handlers::{
        health,
        posts::{create_posts, delete_post, upload_image},
        users::{all_users, create_user, get_config},
    },
    utils::app_state::AppState,
};

pub fn create_routes(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health::health_check))
        .route("/user", get(all_users))
        .route("/user", post(create_user))
        .route("/config", get(get_config))
        .route("/post/image", post(upload_image))
        .route("/post", post(create_posts))
        .route("/post/delete", delete(delete_post))
        // Disable the default 2MB limit and set a new one (4MB)
        .layer(DefaultBodyLimit::disable())
        .layer(RequestBodyLimitLayer::new(4 * 1024 * 1024))
        .nest_service("/images", ServeDir::new("images"))
        .with_state(state)
}
