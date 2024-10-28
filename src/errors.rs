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
    #[error("Unexpected error {0}")]
    Unexpected(String),

    #[error("Validation error: {0}")]
    Validaion(String),

    #[error("License error: {0}")]
    License(String),

    #[error("Not accessible")]
    NotAccessible(),

    #[error("Unauthorized registration access")]
    Unauthorized(),

    #[error("Database connection error: {0}")]
    DatabaseConnection(#[from] deadpool_diesel::PoolError),

    #[error("Database interaction error: {0}")]
    DatabaseInteraction(#[from] deadpool_diesel::InteractError),

    #[error("Database query error: {0}")]
    DatabaseQuery(#[from] diesel::result::Error),
}

impl IntoResponse for APIError {
    fn into_response(self) -> Response {
        error!("API Error occurred: {}", self);

        let (status_code, error_response) = match &self {
            APIError::Unexpected(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                ApiError {
                    status: "failed".to_string(),
                    reason: "Internal Server Error".to_string(),
                    details: "Please contact our support at https://blockfrost.io".to_string(),
                },
            ),
            APIError::Validaion(_) => (
                StatusCode::BAD_REQUEST,
                ApiError {
                    status: "failed".to_string(),
                    reason: "Provided fields are not valid".to_string(),
                    details: self.to_string(),
                },
            ),
            APIError::License(address) => (
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
            APIError::DatabaseConnection(_) | APIError::DatabaseQuery(_) | APIError::DatabaseInteraction(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                ApiError {
                    status: "failed".to_string(),
                    reason: "Database error".to_string(),
                    details: "An error occurred while accessing the database.".to_string(),
                },
            ),
        };

        (status_code, Json(error_response)).into_response()
    }
}
