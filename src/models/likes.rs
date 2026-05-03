use diesel::prelude::*;
use uuid::Uuid;

use super::{posts::Post, users::User};

#[derive(Queryable, Selectable, Insertable, Identifiable, Associations, Debug)]
#[diesel(belongs_to(User))]
#[diesel(belongs_to(Post))]
#[diesel(table_name = crate::schema::likes)]
#[diesel(primary_key(user_id, post_id))]
pub struct Like {
    pub user_id: Uuid,
    pub post_id: Uuid,
    pub created_at: chrono::NaiveDateTime,
}
