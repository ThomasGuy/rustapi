use axum::{
    routing::{delete, get, post},
    Router,
};

use crate::{
    handlers::posts::{all_posts, create_posts, delete_post, get_my_posts, upload_image},
    utils::AppState,
};

pub fn post_routes() -> Router<AppState> {
    Router::new()
        .route("/all_my_posts", post(get_my_posts))
        .route("/image", post(upload_image))
        .route("/", post(create_posts))
        .route("/all", get(all_posts))
        .route("/delete", delete(delete_post))
}
