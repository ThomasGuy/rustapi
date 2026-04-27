use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};

use super::db_error::{ApiError, DbError};

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Database error: {0}")]
    Db(#[from] DbError), // Wraps your existing enum

    #[error("Validation failed: {0}")]
    Validation(String), // For things like "image_url_type must be relative/absolute"

    #[error("Unauthorized: {0}")]
    Unauthorized(String), // For login/Auth

    #[error("Forbidden")]
    Forbidden, // User is logged in but doesn't own this resource
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            AppError::Db(db_err) => db_err.into_response(),
            // Handle NEW high-level API errors here
            AppError::Unauthorized(msg) => {
                let status = StatusCode::UNAUTHORIZED;
                (
                    status,
                    Json(ApiError {
                        error: msg,
                        status: status.as_u16(),
                    }),
                )
                    .into_response()
            }
            AppError::Validation(msg) => {
                let status = StatusCode::BAD_REQUEST;
                (
                    status,
                    Json(ApiError {
                        error: msg,
                        status: status.as_u16(),
                    }),
                )
                    .into_response()
            }
            AppError::Forbidden => {
                let status = StatusCode::FORBIDDEN;
                (
                    status,
                    Json(ApiError {
                        error: "Forbidden".into(),
                        status: status.as_u16(),
                    }),
                )
                    .into_response()
            }
            // Catch-all for AppError variants like JoinError if you added them to AppError too
            // _ => (
            //     StatusCode::INTERNAL_SERVER_ERROR,
            //     "An unexpected error occurred",
            // )
            .into_response(),
        }
    }
}
