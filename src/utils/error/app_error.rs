use super::{ApiError, DbError};
use axum::{
    extract::rejection::JsonRejection,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use jsonwebtoken::errors::ErrorKind;
use tracing::{error, info, warn};

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Database error: {0}")]
    Db(#[from] DbError), // Wraps your existing enum

    #[error("Validation failed: {0}")]
    Validation(String), // For things like "image_url_type must be relative/absolute"

    #[error("Unauthorized: {0}")]
    Auth(String), // For login/Auth

    #[error("Forbidden")]
    Forbidden(String), // User is logged in but doesn't own this resource

    #[error("Multipart malformed: {0}")]
    MultipartError(#[from] axum::extract::multipart::MultipartError),

    #[error("JWT error: {0}")]
    JwtError(#[from] jsonwebtoken::errors::Error),

    #[error("Missing token cookie")]
    MissingCookie,

    #[error("Internal error: {0}")]
    Internal(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            AppError::Db(db_err) => {
                // Log database failures as errors on your VPS logs
                error!(target: "server::database", "Database exception encountered: {:?}", db_err);
                db_err.into_response()
            }

            other_errors => {
                let (status, message) = match &other_errors {
                    AppError::JwtError(inner_err) => match inner_err.kind() {
                        ErrorKind::ExpiredSignature => {
                            // Warn level indicates an expired session that requires rotation
                            warn!("JWT lifetime expired: User needs token rotation");
                            (StatusCode::UNAUTHORIZED, "Token expired".into())
                        }
                        ErrorKind::InvalidToken | ErrorKind::InvalidSignature => {
                            // High priority warning for potential payload tampering or bad clients
                            warn!(
                                target: "server::security",
                                "Security warning: Tampered or invalid JWT signature detected: {:?}",
                                inner_err
                            );
                            (StatusCode::UNAUTHORIZED, "Malformed Token".into())
                        }
                        _ => {
                            error!("Unexpected JWT framework parsing error: {:?}", inner_err);
                            (StatusCode::BAD_REQUEST, "Invalid auth payload.".into())
                        }
                    },

                    AppError::Auth(msg) => {
                        info!("Authentication failed: {:?}", msg);
                        (StatusCode::UNAUTHORIZED, format!("Token invalid. {msg}"))
                    }

                    AppError::MissingCookie => {
                        info!("Authentication failed: Missing refresh token cookie");
                        (StatusCode::UNAUTHORIZED, "No session found".into())
                    }

                    AppError::Validation(msg) => {
                        info!("Validation failed: {}", msg);
                        (StatusCode::BAD_REQUEST, format!("Validation failed: {msg}"))
                    }

                    AppError::Forbidden(err) => {
                        info!("User does not have access to this resource: {}", err);
                        (StatusCode::FORBIDDEN, format!("Forbidden resource: {err}"))
                    }

                    AppError::MultipartError(err) => {
                        info!("Multipart error parsing payload: {:?}", err);
                        (
                            StatusCode::BAD_REQUEST,
                            format!("Malformed response: {err}"),
                        )
                    }

                    AppError::Internal(msg) => {
                        error!("Internal server error: {:?}", msg);
                        (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            "Internal server error".into(),
                        )
                    }

                    _ => {
                        error!("Unhandled application runtime error: {:?}", other_errors);
                        (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            "An unexpected error occured".into(),
                        )
                    }
                };

                let body = Json(ApiError {
                    error: message,
                    status: status.as_u16(),
                });
                (status, body).into_response()
            }
        }
    }
}

impl From<JsonRejection> for AppError {
    fn from(rejection: JsonRejection) -> Self {
        // rejection.body_text() contains the Serde error message
        // like "unknown variant `other`, expected `relative` or `absolute`"
        Self::Validation(rejection.body_text())
    }
}
