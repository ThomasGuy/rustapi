use crate::schema::users;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Queryable, Selectable, Serialize, Identifiable, PartialEq)]
#[diesel(table_name = users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub username: String,
    #[serde(skip_serializing)] // Never expose password hash in API responses
    pub password_hash: String,
    pub display_name: Option<String>,
    pub bio: Option<String>,
    pub avatar_url: Option<String>,
    pub is_active: bool,
    pub is_admin: bool,
    pub email_verified_at: Option<NaiveDateTime>,
    pub last_login_at: Option<NaiveDateTime>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Insertable, Deserialize)]
#[diesel(table_name = users)]
pub struct NewUser {
    pub username: String,
    pub email: String,
    pub password_hash: String,
    // pub display_name: Option<String>,
    // pub bio: Option<String>,
    // pub avatar_url: Option<String>,
    pub is_admin: bool,
}

#[derive(Debug, Insertable, Serialize, Deserialize)]
#[diesel(table_name = users)]
pub struct DisplayUser {
    pub id: Uuid,
    pub email: String,
    pub username: String,
    pub last_login_at: Option<NaiveDateTime>,
    pub created_at: NaiveDateTime,
}
