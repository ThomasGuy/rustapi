use diesel::prelude::*;
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use std::{collections::HashSet, fs, time::Duration};
use tokio::time;
use tracing::{error, info, info_span};
use uuid::Uuid;

use crate::db::{DbConnection, DbPool};
use crate::schema::posts;
use crate::utils::DbError;

// This macro reads your migrations at compile time
pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

pub fn run_migrations(
    conn: &mut impl MigrationHarness<diesel::pg::Pg>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    // This will run all pending migrations
    conn.run_pending_migrations(MIGRATIONS)?;
    Ok(())
}

pub async fn clean_image_folder(pool: DbPool) {
    let mut interval = time::interval(Duration::from_secs(3600 * 24)); // Run every day

    loop {
        interval.tick().await;
        // Everything inside this span will be tagged with 'image_cleanup'
        // and a unique ID for this specific run.
        let span = info_span!("image_cleanup", cleanup_id = %Uuid::new_v4());
        let _enter = span.enter();
        let pool_cloned: DbPool = pool.clone();

        // task::spawn_blocking returns Result<Result<T, DbError>, JoinError>>
        let result = tokio::task::spawn_blocking(move || {
            let mut conn: DbConnection = pool_cloned.get().map_err(DbError::PoolError)?;
            let image_urls = posts::table
                .select(posts::image_url)
                .load::<String>(&mut conn)?;

            // Convert to HashSet for instant lookups
            let image_set: HashSet<String> = image_urls.into_iter().collect();

            for entry in fs::read_dir("./images")? {
                let entry = entry?;
                let path = entry.path();
                if path.is_file() {
                    // Assume filename maps to URL in DB
                    let file_name = path.file_name().unwrap().to_string_lossy().into_owned();
                    if !image_set.contains(&file_name) {
                        fs::remove_file(path)?; // Delete file if not in set
                    }
                }
            }
            Ok::<(), DbError>(()) // Explicitly tell the closure to return your enum
        })
        .await;

        // Handle the Result of the JoinHandle AND the internal Result
        match result {
            Ok(Ok(_)) => info!("Daily image cleanup successful"),
            Ok(Err(e)) => {
                error!("Cleanup logic failed: {}. Will retry in 24h.", e)
            }
            Err(e) => {
                error!("Worker thread panicked: {}. Will retry in 24h.", e)
            }
        }
    } // Span drops here automatically
}

pub async fn clean_expired_tokens(pool: DbPool) {
    let mut interval = time::interval(Duration::from_secs(3600)); // Run every hour

    loop {
        interval.tick().await;

        // Everything inside this span will be tagged with 'image_cleanup'
        // and a unique ID for this specific run.
        let span = info_span!("token_cleanup", cleanup_id = %Uuid::new_v4());
        let _enter = span.enter();
        let pool_cloned: DbPool = pool.clone();

        // Offload the blocking Diesel code to a dedicated thread pool
        let result = tokio::task::spawn_blocking(move || {
            let mut conn: DbConnection = pool_cloned.get().map_err(DbError::PoolError)?;
            use crate::schema::refresh_tokens::dsl::*;
            let count = diesel::delete(refresh_tokens.filter(expires_at.lt(diesel::dsl::now)))
                .execute(&mut conn)
                .map_err(DbError::from)?;
            Ok::<usize, DbError>(count)
        })
        .await;

        match result {
            Ok(Ok(count)) => info!("Cleaned {:?} tokens", count),
            Ok(Err(e)) => {
                error!("DB error: {}. Retrying in 1h.", e)
            }
            Err(e) => {
                error!("Thread panic: {}. Retrying in 1h.", e)
            }
        }
    } // Span drops here automatically
}
