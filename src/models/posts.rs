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
    pub sanity_asset_id: String,
    pub view_count: i32,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Insertable, Deserialize)]
#[diesel(table_name = posts)]
pub struct NewPost {
    pub user_id: Uuid,
    pub caption: Option<String>,
    pub username: String,
    pub sanity_asset_id: String,
}

#[derive(Debug, AsChangeset, Deserialize)]
#[diesel(table_name = posts)]
pub struct UpdatePost {
    pub caption: Option<String>,
    pub sanity_asset_id: String,
}
