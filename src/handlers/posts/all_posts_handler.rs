use axum::{extract::State, Json};
use chrono::NaiveDateTime;
use diesel::prelude::*;
use diesel::RunQueryDsl;
use serde::Serialize;
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::schema::{comments, posts};
use crate::{
    db::{get_connection, DbConnection},
    models::{comments::Comment, posts::Post},
    utils::{AppResult, AppState, DbError},
};

#[derive(Serialize)]
pub struct UserSummary {
    pub username: String,
}

#[derive(Serialize)]
pub struct PostResponse {
    // Frontend Interface
    pub id: Uuid,
    pub image_url: String,
    pub image_url_type: String,
    pub caption: Option<String>,
    pub user_id: Uuid,
    pub user: UserSummary, // Matches TS: user: { username }
    pub timestamp: NaiveDateTime,
    pub comments: Vec<IComment>, // Matches TS: IComment[]
}

#[derive(Serialize)]
pub struct IComment {
    // Frontend interface
    pub id: Uuid,
    pub comment: String, // Matches TS 'text'
    pub username: String,
    pub timestamp: NaiveDateTime,
}

async fn get_posts_reponse(
    posts_data: Vec<Post>,
    state: &AppState,
) -> AppResult<Vec<PostResponse>> {
    let mut conn: DbConnection = get_connection(&state.pool).await?;

    // 1. Fetch Comments for posts_data
    let post_ids: Vec<Uuid> = posts_data.iter().map(|p| p.id).collect();
    let all_comments = comments::table
        .filter(comments::post_id.eq_any(post_ids))
        .order(comments::created_at.desc())
        .load::<Comment>(&mut conn)
        .map_err(DbError::from)?;

    // 2. Group comments by post_id
    let mut comments_map: std::collections::HashMap<Uuid, Vec<IComment>> =
        std::collections::HashMap::new();
    for c in all_comments {
        comments_map.entry(c.post_id).or_default().push(IComment {
            id: c.id,
            comment: c.comment,
            username: c.username,
            timestamp: c.created_at,
        });
    }

    // 3. Map to Frontend Interface
    let response = posts_data
        .into_iter()
        .map(|p| PostResponse {
            id: p.id,
            image_url: p.image_url,
            image_url_type: p.image_url_type,
            caption: p.caption,
            user_id: p.user_id,
            user: UserSummary {
                username: p.username,
            }, // Map 'p.username' to nested object
            timestamp: p.created_at,
            comments: comments_map.remove(&p.id).unwrap_or_default(),
        })
        .collect();

    Ok(response)
}

pub async fn all_posts(State(state): State<AppState>) -> AppResult<Json<Vec<PostResponse>>> {
    let mut conn: DbConnection = get_connection(&state.pool).await?;

    // 1. Fetch Posts
    let posts_data = posts::table
        .order(posts::created_at.desc())
        .load::<Post>(&mut conn)
        .map_err(DbError::from)?;

    tracing::info!(num_posts=%posts_data.len(), "all_posts success");
    let response = get_posts_reponse(posts_data, &state).await?;
    Ok(Json(response))
}

pub async fn get_my_posts(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> AppResult<Json<Vec<PostResponse>>> {
    let mut conn: DbConnection = get_connection(&state.pool).await?;

    // Fetch posts
    let my_posts = posts::table
        .filter(posts::user_id.eq(user.id))
        .order(posts::created_at.desc())
        .load::<Post>(&mut conn)
        .map_err(DbError::from)?;

    let response = get_posts_reponse(my_posts, &state).await?;

    Ok(Json(response))
}
