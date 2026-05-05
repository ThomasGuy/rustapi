use axum::{
    routing::{delete, get},
    Router,
};

use crate::{
    handlers::admin::{admin_users, delete_user_admin},
    utils::AppState,
};

pub fn admin_routes() -> Router<AppState> {
    Router::new()
        .route("/users", get(admin_users))
        .route("/user/{id}", delete(delete_user_admin))
}
