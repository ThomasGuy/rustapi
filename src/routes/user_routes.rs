use axum::{
    routing::{get, post},
    Router,
};

use crate::{
    handlers::{
        login::{login, logout, refresh_handler},
        users::{all_users, register},
    },
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
