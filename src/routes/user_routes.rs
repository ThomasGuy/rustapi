use axum::{
    routing::{get, patch, post},
    Router,
};

use crate::{
    handlers::users::{
        all_users, login, logout, refresh_handler, register, update_profile, user_check,
    },
    utils::AppState,
};

pub fn user_routes() -> Router<AppState> {
    Router::new()
        .route("/", get(all_users))
        .route("/check", get(user_check))
        .route("/signup", post(register))
        .route("/login", post(login))
        .route("/logout", post(logout))
        .route("/refresh", post(refresh_handler))
        .route("/update", patch(update_profile))
}
