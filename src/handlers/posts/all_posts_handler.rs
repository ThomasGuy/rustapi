use std::collections::HashMap;

use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::NaiveDateTime;
use diesel::prelude::*;
use diesel::RunQueryDsl;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::schema::{comments, likes, posts, users};
use crate::utils::AppError;
use crate::{auth::CurrentUser, handlers::posts::post_handler::SanityImage};
use crate::{
    db::{get_connection, DbConnection},
    models::{comments::Comment, likes::Like, posts::Post},
    utils::{AppResult, AppState, DbError},
};

#[derive(Debug, Deserialize)]
pub struct PaginationQuery {
    pub offset: Option<i64>,
}

// Global fallback defaults for safe database protection
const DEFAULT_ALL_LIMIT: i64 = 20;
const DEFAULT_USER_LIMIT: i64 = 60;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserSummary {
    pub username: String,
    pub avatar_url: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PostResponse {
    pub id: Uuid,
    pub sanity_image: SanityImage,
    pub caption: Option<String>,
    pub user_id: Uuid,
    pub user: UserSummary, // Matches TS: user: UserSummary;
    pub created_at: NaiveDateTime,
    pub comments: Vec<IComment>, // Matches TS: IComment[]
    pub likes_count: i64,
    pub has_liked: bool,
}

#[derive(Serialize)]
pub struct IComment {
    pub id: Uuid,
    pub comment: String, // Matches TS 'text'
    pub username: String,
    pub timestamp: NaiveDateTime,
}

async fn get_posts_reponse(
    posts_data: Vec<Post>,
    state: &AppState,
    current_user_id: Uuid,
) -> AppResult<Vec<PostResponse>> {
    // This stops Diesel from executing invalid empty SQL operations.
    if posts_data.is_empty() {
        return Ok(Vec::new());
    }

    let mut conn: DbConnection = get_connection(&state.pool).await?;
    let post_ids: Vec<Uuid> = posts_data.iter().map(|p| p.id).collect();

    // 🚀 NEW BATCH STEP: Extract all distinct author user IDs from the posts list
    let author_ids: Vec<Uuid> = posts_data.iter().map(|p| p.user_id).collect();

    // Fetch only the username and avatar columns from the users table for these specific authors
    let authors_avatars = users::table
        .filter(users::id.eq_any(&author_ids))
        .select((users::id, users::avatar_url))
        .load::<(Uuid, Option<String>)>(&mut conn)
        .map_err(DbError::from)?;

    // Group avatars into a lookup map: HashMap<UserId, AvatarUrlString>
    let avatars_map: HashMap<Uuid, Option<String>> = authors_avatars.into_iter().collect();

    // 1. Fetch ALL likes for these posts in one go
    let all_likes = likes::table
        .filter(likes::post_id.eq_any(&post_ids))
        .load::<Like>(&mut conn)
        .map_err(DbError::from)?;

    // 2. Map likes for easy lookup: HashMap<PostId, Vec<UserId>>
    let mut likes_map: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
    for l in all_likes {
        likes_map.entry(l.post_id).or_default().push(l.user_id);
    }

    // 3. Fetch Comments for posts_data
    let all_comments = comments::table
        .filter(comments::post_id.eq_any(&post_ids))
        .order(comments::created_at.desc())
        .load::<Comment>(&mut conn)
        .map_err(DbError::from)?;

    // 4. Group comments by post_id
    let mut comments_map: HashMap<Uuid, Vec<IComment>> = HashMap::new();
    for c in all_comments {
        comments_map.entry(c.post_id).or_default().push(IComment {
            id: c.id,
            comment: c.comment,
            username: c.username,
            timestamp: c.created_at,
        });
    }

    // 5. Map to Frontend Interface
    let response = posts_data
        .into_iter()
        .map(|p| {
            let post_likes = likes_map.get(&p.id);
            let likes_count = post_likes.map(|v| v.len()).unwrap_or(0) as i64;
            let has_liked = post_likes
                .map(|v| v.contains(&current_user_id))
                .unwrap_or(false);

            // 🚀 LOOK UP AVATAR: Look up the avatar string using the post author's user_id
            // If the map lookup returns None, it gracefully defaults to None (null in JSON)
            let author_avatar = avatars_map.get(&p.user_id).cloned().flatten();

            PostResponse {
                id: p.id,
                user_id: p.user_id,
                caption: p.caption,
                sanity_image: p.sanity_image,
                user: UserSummary {
                    username: p.username,
                    avatar_url: author_avatar,
                },
                created_at: p.created_at,
                comments: comments_map.remove(&p.id).unwrap_or_default(),
                likes_count,
                has_liked,
            }
        })
        .collect();

    Ok(response)
}

// GET /post/all
#[axum::debug_handler]
pub async fn all_posts(
    State(state): State<AppState>,
    Query(pagination): Query<PaginationQuery>,
    auth_result: Result<CurrentUser, AppError>,
) -> AppResult<Json<Vec<PostResponse>>> {
    let mut conn: DbConnection = get_connection(&state.pool).await?;

    // Fallback: Default to offset 0 if the UI didn't specify one
    let page_offset = pagination.offset.unwrap_or(0);

    // 1. Fetch Posts
    let posts_data = posts::table
        .order(posts::created_at.desc())
        .limit(DEFAULT_ALL_LIMIT)
        .offset(page_offset)
        .select(Post::as_select())
        .load::<Post>(&mut conn)
        .map_err(DbError::from)?;

    // Map the result: If OK, use user_id; if ERR, use nil (guest mode)
    let user_id = match auth_result {
        Ok(CurrentUser(user)) => user.id,
        Err(_) => Uuid::nil(),
    };

    tracing::info!(num_posts=%posts_data.len(), "all_posts success");
    let response = get_posts_reponse(posts_data, &state, user_id).await?;

    Ok(Json(response))
}

// GET /post/user/{username}
pub async fn get_user_posts(
    State(state): State<AppState>,
    viewer: Result<CurrentUser, AppError>,
    Path(username): Path<String>,
    Query(pagination): Query<PaginationQuery>,
) -> AppResult<Json<Vec<PostResponse>>> {
    let mut conn: DbConnection = get_connection(&state.pool).await?;

    let page_offset = pagination.offset.unwrap_or(0);

    // 1. Get the posts for the target user
    let user_posts = posts::table
        .filter(posts::username.eq(&username))
        .order(posts::created_at.desc())
        .limit(DEFAULT_USER_LIMIT)
        .offset(page_offset)
        .select(Post::as_select())
        .load::<Post>(&mut conn)
        .map_err(DbError::from)?;

    // 2. Identify the viewer (for the 'has_liked' heart)
    let viewer_id = match viewer {
        Ok(CurrentUser(v)) => v.id,
        Err(_) => Uuid::nil(), // Guests see white hearts
    };

    let response = get_posts_reponse(user_posts, &state, viewer_id).await?;

    Ok(Json(response))
}
