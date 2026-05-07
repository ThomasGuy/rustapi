use axum::{
    routing::{delete, get, post},
    Router,
};

use crate::{
    handlers::posts::{
        all_posts, create_comment, create_posts, delete_post, get_user_posts, toggle_like,
        upload_image,
    },
    utils::AppState,
};

pub fn post_routes() -> Router<AppState> {
    Router::new()
        .route("/comment", post(create_comment))
        .route("/user/{username}", get(get_user_posts))
        .route("/image", post(upload_image))
        .route("/create", post(create_posts))
        .route("/all", get(all_posts))
        .route("/delete/{id}", delete(delete_post))
        .route("/like/{id}", post(toggle_like))
}
