use crate::schema::refresh_tokens;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde::Serialize;
use uuid::Uuid;

use super::users::User;

#[derive(Debug, Queryable, Selectable, Serialize, Identifiable, Associations, PartialEq)]
#[diesel(belongs_to(User))]
#[diesel(table_name = refresh_tokens)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct RefreshToken {
    pub id: Uuid,
    pub user_id: Uuid,
    pub token_hash: String,
    pub expires_at: NaiveDateTime,
    pub created_at: Option<NaiveDateTime>,
}
