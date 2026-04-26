use crate::schema::posts;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::users::User;

#[derive(Debug, Clone, Queryable, Selectable, Serialize, Identifiable, Associations)]
#[diesel(belongs_to(User))]
#[diesel(table_name = posts)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Post {
    pub id: Uuid,
    pub user_id: Uuid,
    pub title: String,
    pub slug: String,
    pub content: String,
    pub excerpt: Option<String>,
    pub published: bool,
    pub published_at: Option<NaiveDateTime>,
    pub view_count: i32,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Insertable, Deserialize)]
#[diesel(table_name = posts)]
pub struct NewPost {
    pub user_id: Uuid,
    pub title: String,
    pub slug: String,
    pub content: String,
    pub excerpt: Option<String>,
    pub published: Option<bool>,
    pub published_at: Option<NaiveDateTime>,
}

#[derive(Debug, AsChangeset, Deserialize)]
#[diesel(table_name = posts)]
pub struct UpdatePost {
    pub title: Option<String>,
    pub slug: Option<String>,
    pub content: Option<String>,
    pub excerpt: Option<String>,
    pub published: Option<bool>,
    pub published_at: Option<NaiveDateTime>,
}

// For listings with author info
// #[derive(Debug, Queryable, Serialize)]
// pub struct PostWithAuthor {
//     pub id: Uuid,
//     pub title: String,
//     pub slug: String,
//     pub excerpt: Option<String>,
//     pub published_at: Option<NaiveDateTime>,
//     pub view_count: i32,
//     pub author_username: String,
//     pub author_display_name: Option<String>,
// }
