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
}

impl From<deadpool_diesel::PoolError> for APIError {
    fn from(err: deadpool_diesel::PoolError) -> Self {
        error!("Database connection error: {}", err);
        APIError::UnexpectedError()
    }
}

impl IntoResponse for APIError {
    fn into_response(self) -> Response {
        let error_response = match &self {
            APIError::UnexpectedError() => ApiError {
                status: "failed".to_string(),
                reason: "Internal Server Error".to_string(),
                details: "Please contact our support at https://blockfrost.io".to_string(),
            },
            APIError::ValidationError(_) => ApiError {
                status: "failed".to_string(),
                reason: "Provided fields are not valid".to_string(),
                details: self.to_string(),
            },
            APIError::LicenseError(address) => ApiError {
                status: "failed".to_string(),
                reason: "no_license".to_string(),
                details: format!("Address: {} does not contain the license.", address),
            },
            APIError::NotAccessible() => ApiError {
                status: "failed".to_string(),
                reason: "not_accessible".to_string(),
                details: "The Blockfrost instance is not publically accessible.".to_string(),
            },
        };

        (StatusCode::INTERNAL_SERVER_ERROR, Json(error_response)).into_response()
    }
}
