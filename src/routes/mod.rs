mod admin_routes;
mod post_routes;
mod user_routes;

// use axum::http::header::{AUTHORIZATION, CONTENT_TYPE};
use axum::http::{header::HeaderName, HeaderValue, Method};
use axum::{extract::DefaultBodyLimit, routing::get, Router};
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::{limit::RequestBodyLimitLayer, services::ServeDir};

use crate::{
    handlers::health::health_check,
    utils::{AppState, Environment},
};

use admin_routes::admin_routes;
use post_routes::post_routes;
use user_routes::user_routes;

pub fn create_routes() -> Router<AppState> {
    Router::new()
        .route("/health", get(health_check))
        .nest("/user", user_routes())
        .nest("/post", post_routes())
        .nest("/admin", admin_routes())
        // Disable the default 2MB limit and set a new one (7MB)
        .layer(DefaultBodyLimit::disable())
        .layer(RequestBodyLimitLayer::new(7 * 1024 * 1024))
        .nest_service("/images", ServeDir::new("images"))
}

pub fn generate_cors_layer(environment: Environment) -> CorsLayer {
    let allowed_headers = [
        HeaderName::from_static("authorization"),
        HeaderName::from_static("content-type"),
        HeaderName::from_static("accept"),
    ];

    match environment {
        Environment::Local => {
            // Define your permitted local dev origins
            let allowed_local_origins = ["http://localhost:5173", "http://192.168.1.48:5173"];

            CorsLayer::new()
                // Use the dynamic closure builder to check incoming requests in real-time
                .allow_origin(AllowOrigin::predicate(
                    move |origin: &HeaderValue, _request_parts| {
                        if let Ok(origin_str) = origin.to_str() {
                            return allowed_local_origins.contains(&origin_str);
                        }
                        false
                    },
                ))
                .allow_methods([
                    Method::GET,
                    Method::POST,
                    Method::PUT,
                    Method::DELETE,
                    Method::OPTIONS,
                ])
                .allow_headers(allowed_headers)
                .allow_credentials(true) // Crucial for cookie transmissions
        }
        Environment::Production => CorsLayer::new()
            .allow_origin(AllowOrigin::exact(HeaderValue::from_static(
                "https://yourfrontend.com",
            )))
            .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
            .allow_headers(allowed_headers)
            .allow_credentials(true),
    }
}
