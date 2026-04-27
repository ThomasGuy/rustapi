mod config;
mod db;
mod handlers;
mod models;
mod routes;
mod schema;
mod utils;

use std::sync::{Arc, RwLock};

use axum::{body::Body, http::Request};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;

use db::{get_connection, init_pool, DbConnection, DbPool};
use routes::create_routes;
use utils::app_state::{AppState, TokenKeys};

// This macro reads your migrations at compile time
pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // initialize tracing
    tracing_subscriber::fmt()
        .with_target(false)
        .compact()
        .init();

    // 2. Configure the TraceLayer with a custom span
    let trace_layer = TraceLayer::new_for_http().make_span_with(|request: &Request<Body>| {
        // Generate a unique ID for this specific request
        // let request_id = Uuid::new_v4();

        // Create a span that includes the method, uri, and our custom ID
        tracing::info_span!(
            "http-request",
            method = %request.method(),
            uri = %request.uri(),
            // request_id = %request_id,
        )
    });

    let config = config::AppConfig::from_env();
    let pool: DbPool = init_pool(&config)?;

    // run migrations
    {
        let mut conn: DbConnection = get_connection(&pool).await?;
        run_migrations(&mut conn).map_err(|err| anyhow::anyhow!("Migrations failed: {}", err))?;
    }

    let token = TokenKeys {
        token_key: String::from("a_key"),
    };

    let state = AppState {
        pool: pool.clone(),
        config: Arc::new(config.clone()),
        public_keys: Arc::new(RwLock::new(token)),
    };

    let app = create_routes(state).layer(trace_layer);
    tokio::fs::create_dir_all("./images").await?;

    // Start server
    let addr = format!("{}:{}", config.host, config.port);
    tracing::info!("Server starting on http://{addr}");
    let listener = TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();

    anyhow::Ok(())
}

fn run_migrations(
    conn: &mut impl MigrationHarness<diesel::pg::Pg>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    // This will run all pending migrations
    conn.run_pending_migrations(MIGRATIONS)?;
    Ok(())
}
