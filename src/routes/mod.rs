use axum::{
    extract::DefaultBodyLimit,
    routing::{delete, get, post},
    Router,
};
use tower_http::{limit::RequestBodyLimitLayer, services::ServeDir};

use crate::{
    handlers::{
        comments::create_comment,
        health,
        login::{login, logout},
        posts::{all_posts, create_posts, delete_post, get_my_posts, upload_image},
        refresh::refresh_handler,
        users::{all_users, get_config, register},
    },
    utils::AppState,
};

pub fn create_routes(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health::health_check))
        .route("/user", get(all_users))
        .route("/user", post(register))
        .route("/user/login", post(login))
        .route("/user/logout", post(logout))
        .route("/user/refresh", post(refresh_handler))
        .route("/user/comment", post(create_comment))
        .route("/config", get(get_config))
        .route("/post/image", post(upload_image))
        .route("/post", post(create_posts))
        .route("/user/all_posts", post(get_my_posts))
        .route("/post/all", get(all_posts))
        .route("/post/delete", delete(delete_post))
        // Disable the default 2MB limit and set a new one (4MB)
        .layer(DefaultBodyLimit::disable())
        .layer(RequestBodyLimitLayer::new(4 * 1024 * 1024))
        .nest_service("/images", ServeDir::new("images"))
        .with_state(state)
}
