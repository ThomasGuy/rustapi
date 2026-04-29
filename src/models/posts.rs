use crate::schema::posts;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::users::User;

#[derive(Debug, Queryable, Selectable, Serialize, Identifiable, Associations, PartialEq)]
#[diesel(belongs_to(User))]
#[diesel(table_name = posts)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Post {
    pub id: Uuid,
    pub user_id: Uuid,
    pub caption: Option<String>,
    pub username: String,
    pub image_url: String,
    pub image_url_type: String,
    // pub published: bool,
    // pub published_at: Option<NaiveDateTime>,
    pub view_count: i32,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Insertable, Deserialize)]
#[diesel(table_name = posts)]
pub struct NewPost {
    pub user_id: Uuid,
    // pub title: String,
    pub caption: Option<String>,
    pub username: String,
    pub image_url: String,
    pub image_url_type: String,
    // pub published: Option<bool>,
    // pub published_at: Option<NaiveDateTime>,
}

#[derive(Debug, AsChangeset, Deserialize)]
#[diesel(table_name = posts)]
pub struct UpdatePost {
    // pub title: Option<String>,
    pub caption: Option<String>,
    pub image_url: Option<String>,
    pub image_url_type: Option<String>,
    // pub published: Option<bool>,
    // pub published_at: Option<NaiveDateTime>,
}

// front end display post
// #[derive(Debug, Queryable, Serialize)]
// #[diesel(base_query = posts::table.order_by(posts::created_at.asc()))]
// #[diesel(check_for_backend(diesel::pg::Pg))]
// pub struct PostDisplay {
//     pub id: Uuid,
//     pub image_url: String,
//     pub image_url_type: String,
//     pub caption: Option<String>,
//     pub user_id: Uuid,
//     pub username: String,
//     pub timestamp: NaiveDateTime,
//     pub comments: Vec<CommentDisplay>,
// }
