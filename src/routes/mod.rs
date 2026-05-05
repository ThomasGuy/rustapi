mod post_routes;
mod user_routes;
mod admin_routes;

use axum::{
    extract::DefaultBodyLimit,
    routing::{get, post},
    Router,
};
use tower_http::{limit::RequestBodyLimitLayer, services::ServeDir};

use crate::{
    handlers::{comments::create_comment, health::health_check},
    utils::AppState,
};

use post_routes::post_routes;
use user_routes::user_routes;
use admin_routes::admin_routes;

pub fn create_routes() -> Router<AppState> {
    Router::new()
        .route("/health", get(health_check))
        .route("/comment", post(create_comment))
        .nest("/user", user_routes())
        .nest("/post", post_routes())
        .nest("/admin", admin_routes())
        // Disable the default 2MB limit and set a new one (4MB)
        .layer(DefaultBodyLimit::disable())
        .layer(RequestBodyLimitLayer::new(7 * 1024 * 1024))
        .nest_service("/images", ServeDir::new("images"))
}
