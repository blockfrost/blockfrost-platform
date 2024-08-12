use axum::response::{IntoResponse, Response};
use axum::{http, Json};
use http::StatusCode;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::error;

#[derive(Serialize, Deserialize)]
pub struct ApiError {
    status: String,
    reason: String,
    details: String,
}

#[derive(Error, Debug)]
pub enum APIError {
    #[error("Unexpected error")]
    UnexpectedError(),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("License error: {0}")]
    LicenseError(String),

    #[error("Not accessible")]
    NotAccessible(),

    #[error("Unauthorized registration access")]
    Unauthorized(),
}

impl From<deadpool_diesel::PoolError> for APIError {
    fn from(err: deadpool_diesel::PoolError) -> Self {
        error!("Database connection error: {}", err);
        APIError::UnexpectedError()
    }
}

impl IntoResponse for APIError {
    fn into_response(self) -> Response {
        let (status_code, error_response) = match &self {
            APIError::UnexpectedError() => (
                StatusCode::INTERNAL_SERVER_ERROR,
                ApiError {
                    status: "failed".to_string(),
                    reason: "Internal Server Error".to_string(),
                    details: "Please contact our support at https://blockfrost.io".to_string(),
                },
            ),
            APIError::ValidationError(_) => (
                StatusCode::BAD_REQUEST,
                ApiError {
                    status: "failed".to_string(),
                    reason: "Provided fields are not valid".to_string(),
                    details: self.to_string(),
                },
            ),
            APIError::LicenseError(address) => (
                StatusCode::FORBIDDEN,
                ApiError {
                    status: "failed".to_string(),
                    reason: "no_license".to_string(),
                    details: format!("Address: {} does not contain the license.", address),
                },
            ),
            APIError::NotAccessible() => (
                StatusCode::FORBIDDEN,
                ApiError {
                    status: "failed".to_string(),
                    reason: "not_accessible".to_string(),
                    details: "The Blockfrost instance is not publicly accessible.".to_string(),
                },
            ),
            APIError::Unauthorized() => (
                StatusCode::FORBIDDEN,
                ApiError {
                    status: "failed".to_string(),
                    reason: "unauthorized".to_string(),
                    details: "You are not authorized to access the registration. Please contact our support at https://blockfrost.io".to_string(),
                },
            ),
        };

        (status_code, Json(error_response)).into_response()
    }
}
