pub mod all_posts;
pub mod post_handlers;

pub use {
    all_posts::all_posts,
    post_handlers::{create_posts, delete_post, get_my_posts, upload_image},
};
