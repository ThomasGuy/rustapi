// use axum::{
//     // extract::{Path, Query},
//     routing::{get, post},
//     Router,
// };
// use serde::Deserialize;

// use crate::{
// db::DbPool,
// handlers::users::{all_users, create_user, get_config},
// utils::app_state::AppState,
// };

// pub fn user_routes(state: AppState) -> Router {
//     Router::new()
//         .route("/api/user", get(all_users))
//         .route("/api/user", post(create_user))
//         .route("/config", get(get_config))
//         .with_state(state)
// }
