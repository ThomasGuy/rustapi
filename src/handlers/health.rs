use axum::{http::StatusCode, Json};
use serde_json::{json, Value};

// GET /health
pub async fn health_check() -> (StatusCode, Json<Value>) {
    (
        StatusCode::OK,
        Json(json!({
            "status": "ok",
            "version": env!("CARGO_PKG_VERSION"),
        })),
    )
}
