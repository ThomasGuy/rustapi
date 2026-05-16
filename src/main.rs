pub(crate) mod auth;
pub(crate) mod db;
pub(crate) mod handlers;
pub(crate) mod models;
pub(crate) mod routes;
pub(crate) mod schema;
pub(crate) mod utils;

use axum::{body::Body, http::Request};
use jsonwebtoken::{DecodingKey, EncodingKey};
use std::sync::Arc;
use tokio::net::TcpListener;
use tower_cookies::CookieManagerLayer;
use tower_http::trace::TraceLayer;
use uuid::Uuid;

use auth::claims::TokenKeys;
use db::{get_connection, init_pool, DbConnection, DbPool};
use routes::{create_routes, generate_cors_layer};
use utils::{
    workers::{clean_expired_tokens, run_migrations},
    AppState,
};

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

    let cors_middleware = generate_cors_layer(state.config.app_env);

    let app = create_routes()
        .with_state(state)
        .layer(trace_layer)
        .layer(cors_middleware)
        .layer(CookieManagerLayer::new());

    tokio::fs::create_dir_all("./images").await?;
    tokio::spawn(clean_expired_tokens(pool.clone()));
    // tokio::spawn(clean_image_folder(pool.clone()));

    // Start server
    let addr = format!("{}:{}", config.host, config.port);
    tracing::info!("Server starting on http://{addr}");
    let listener = TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();

    anyhow::Ok(())
}
