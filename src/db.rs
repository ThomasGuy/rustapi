use diesel::pg::PgConnection;
use diesel::r2d2::ConnectionManager;
use diesel::r2d2::{Pool, PooledConnection};

use crate::config::AppConfig;
use crate::utils::db_error::DbError;

// Type alias for cleaner code
pub type DbPool = Pool<ConnectionManager<PgConnection>>;
pub type DbConnection = PooledConnection<ConnectionManager<PgConnection>>;

// Initialize the connection pool
pub fn init_pool(config: &AppConfig) -> Result<DbPool, DbError> {
    let manager = ConnectionManager::<PgConnection>::new(config.database_url.clone());

    Pool::builder()
        .max_size(12) // Maximum connections in the pool
        .min_idle(Some(2)) // Minimum idle connections to maintain
        .connection_timeout(std::time::Duration::from_secs(5))
        .idle_timeout(Some(std::time::Duration::from_secs(300)))
        .build(manager)
        .map_err(DbError::PoolError)
}

// Helper to get a connection from the pool
pub async fn get_connection(pool: &DbPool) -> Result<DbConnection, DbError> {
    pool.get().map_err(DbError::PoolError)
}
