use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};

/// Generic error response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    pub error: String,
    pub status: u16,
}

// Custom error type for database operations
#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Database query error: {0}")]
    DatabaseError(#[from] diesel::result::Error),

    #[error("Connection manager error: {0}")]
    ConnectionError(#[from] diesel::r2d2::Error),

    #[error("Pool timeout or initialization error: {0}")]
    PoolError(#[from] diesel::r2d2::PoolError),

    #[error("Internal task error: {0}")]
    JoinError(#[from] tokio::task::JoinError),

    #[error("file upload failed: {0}")]
    UploadError(#[from] std::io::Error),
}

impl IntoResponse for DbError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            DbError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            DbError::DatabaseError(err) => match err {
                diesel::result::Error::NotFound => {
                    (StatusCode::NOT_FOUND, "Resource not found.".to_string())
                }
                diesel::result::Error::DatabaseError(
                    diesel::result::DatabaseErrorKind::UniqueViolation,
                    _,
                ) => (StatusCode::CONFLICT, "Resource already exists".into()),
                _ => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Database error: {err}"),
                ),
            },
            DbError::ConnectionError(err) => (
                StatusCode::SERVICE_UNAVAILABLE,
                format!("Connection manager error: {err}"),
            ),
            DbError::PoolError(err) => (
                StatusCode::SERVICE_UNAVAILABLE,
                format!("Connection pool error: {err}"),
            ),
            DbError::JoinError(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Internal task error: {err}"),
            ),
            DbError::UploadError(err) => (
                StatusCode::BAD_REQUEST,
                format!("I/O file upload error: {err}"),
            ),
        };

        tracing::error!("Request error {status}: {message}");

        let body = Json(ApiError {
            error: message,
            status: status.as_u16(),
        });

        (status, body).into_response()
    }
}
