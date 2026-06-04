mod admin_routes;
mod post_routes;
mod user_routes;

use axum::http::{header::HeaderName, HeaderValue, Method};
use axum::{routing::get, Router};
use tower_http::cors::{AllowOrigin, CorsLayer};

use crate::{
    handlers::health::health_check,
    utils::{AppState, Environment},
};

pub fn create_routes() -> Router<AppState> {
    Router::new()
        .route("/health", get(health_check))
        .nest("/user", user_routes::user_routes())
        .nest("/post", post_routes::post_routes())
        .nest("/admin", admin_routes::admin_routes())
}

pub fn generate_cors_layer(environment: Environment) -> CorsLayer {
    let allowed_headers = [
        HeaderName::from_static("authorization"),
        HeaderName::from_static("content-type"),
        HeaderName::from_static("accept"),
        HeaderName::from_static("cookie"),
    ];

    match environment {
        Environment::Local => {
            // Define your permitted local dev origins
            let allowed_local_origins = [
                "http://localhost:5173",
                "http://192.168.1.9:5173",
                "http://localhost:4173",
                "http://192.168.1.9:4173",
                "http://10.255.255.254:5173",
                "http://172.22.143.216:5173",
            ];

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
        Environment::Production => {
            // Define your permitted production web layout entry points
            let allowed_production_origins = [
                "http://213.171.209.232",
                "http://twguy.co.uk",
                "https://twguy.co.uk",
            ];

            CorsLayer::new()
                // Use a dynamic predicate closure to validate the incoming origin in real-time
                .allow_origin(AllowOrigin::predicate(
                    move |origin: &HeaderValue, _request_parts| {
                        if let Ok(origin_str) = origin.to_str() {
                            return allowed_production_origins.contains(&origin_str);
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
                .allow_credentials(true)
        }
    }
}
