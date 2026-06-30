use diesel::prelude::*;

use std::time::Duration;
use tokio::time;
use tracing::{error, info, info_span};
use uuid::Uuid;

use crate::db::{DbConnection, DbPool};
use crate::utils::DbError;

pub async fn clean_expired_tokens(pool: DbPool) {
    let mut interval = time::interval(Duration::from_secs(3600 * 24)); // Run every day

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
                error!("DB error: {}. Retrying in 1 day.", e)
            }
            Err(e) => {
                error!("Thread panic: {}. Retrying in 1 day.", e)
            }
        }
    } // Span drops here automatically
}
