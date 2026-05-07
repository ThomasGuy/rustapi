use axum::{
    routing::{get, post},
    Router,
};

use crate::{
    handlers::users::{all_users, login, logout, refresh_handler, register},
    utils::AppState,
};

pub fn user_routes() -> Router<AppState> {
    Router::new()
        .route("/", get(all_users))
        .route("/signup", post(register))
        .route("/login", post(login))
        .route("/logout", post(logout))
        .route("/refresh", post(refresh_handler))
}
