pub(crate) mod auth;
pub(crate) mod db;
pub(crate) mod handlers;
pub(crate) mod models;
pub(crate) mod routes;
pub(crate) mod schema;
pub(crate) mod utils;

use diesel::prelude::*;
use std::time::Duration;
use std::{fs, sync::Arc};
use tokio::time;

use axum::{
    body::Body,
    http::{
        header::{AUTHORIZATION, CONTENT_TYPE},
        Request,
    },
};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use jsonwebtoken::{DecodingKey, EncodingKey};
use tokio::net::TcpListener;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use uuid::Uuid;

use auth::claims::TokenKeys;
use db::{get_connection, init_pool, DbConnection, DbPool};
use routes::create_routes;
use schema::posts;
use utils::{AppState, DbError};

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
        let request_id = Uuid::new_v4();

        // Create a span that includes the method, uri, and our custom ID
        tracing::info_span!(
            "http-request",
            method = %request.method(),
            uri = %request.uri(),
            request_id = %request_id,
        )
    });

    let cors = CorsLayer::new()
        .allow_origin(tower_http::cors::Any) // Allows phone, tablet, and PC
        .allow_methods(tower_http::cors::Any)
        .allow_headers([AUTHORIZATION, CONTENT_TYPE]);

    // Define the CORS configuration
    // let cors = CorsLayer::new()
    //     // Allow specific origin; use Any for development
    //     .allow_origin(
    //         "http://localhost:5173"
    //             .parse::<http::HeaderValue>()
    //             .unwrap(),
    //     )
    //     .allow_methods([Method::GET, Method::POST, Method::PATCH, Method::DELETE])
    //     .allow_headers([CONTENT_TYPE, AUTHORIZATION])
    //     .allow_credentials(true);

    let config = utils::AppConfig::from_env();
    let pool: DbPool = init_pool(&config)?;

    // run migrations
    {
        let mut conn: DbConnection = get_connection(&pool).await?;
        run_migrations(&mut conn).map_err(|err| anyhow::anyhow!("Migrations failed: {}", err))?;
    }

    let keys = TokenKeys {
        encoding_key: EncodingKey::from_secret(config.secret_key.as_bytes()),
        decoding_key: DecodingKey::from_secret(config.secret_key.as_bytes()),
    };

    let state = AppState {
        pool: pool.clone(),
        config: Arc::new(config.clone()),
        public_keys: Arc::new(keys),
    };

    let app = create_routes()
        .with_state(state)
        .layer(trace_layer)
        .layer(cors);

    tokio::fs::create_dir_all("./images").await?;
    tokio::spawn(clean_expired_tokens(pool.clone()));
    tokio::spawn(clean_image_folder(pool.clone()));

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

async fn clean_image_folder(pool: DbPool) -> Result<(), DbError> {
    let mut interval = time::interval(Duration::from_secs(3600 * 24)); // Run every day

    loop {
        interval.tick().await;
        let mut conn: DbConnection = get_connection(&pool).await?;
        let image_urls = posts::table
            .select(posts::image_url)
            .load::<String>(&mut conn)?;

        for entry in fs::read_dir("./images")? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                // Assume filename maps to URL in DB
                let file_name = path.file_name().unwrap().to_string_lossy().into_owned();
                if !image_urls.contains(&file_name) {
                    println!("Deleting: {:?}", path);
                    fs::remove_file(path)?; // Delete file if not in set
                }
            }
        }
    }
}

async fn clean_expired_tokens(pool: DbPool) {
    let mut interval = time::interval(Duration::from_secs(3600)); // Run every hour

    loop {
        interval.tick().await;
        let mut conn: DbConnection = get_connection(&pool)
            .await
            .expect("Worker couldn't get connection");
        use crate::schema::refresh_tokens::dsl::*;
        let result = diesel::delete(refresh_tokens.filter(expires_at.lt(diesel::dsl::now)))
            .execute(&mut conn);
        match result {
            Ok(count) => println!("Cleaned up {} expired tokens", count),
            Err(e) => eprintln!("Error cleaning tokens: {}", e),
        }
    }
}
