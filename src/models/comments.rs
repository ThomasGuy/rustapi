use crate::schema::comments;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::posts::Post;

#[derive(Debug, Queryable, Selectable, Serialize, Identifiable, Associations, PartialEq)]
#[diesel(belongs_to(Post))]
#[diesel(table_name = comments)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Comment {
    pub id: Uuid,
    pub post_id: Uuid,
    pub user_id: Uuid,
    pub username: String,
    pub comment: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Insertable, Deserialize)]
#[diesel(table_name = comments)]
pub struct NewComment {
    pub post_id: Uuid,
    pub user_id: Uuid,
    pub username: String,
    pub comment: String,
}

#[derive(Debug, AsChangeset, Deserialize)]
#[diesel(table_name = comments)]
pub struct UpdateComment {
    pub comment: Option<String>,
}
