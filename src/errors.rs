use axum::response::{IntoResponse, Response};
use axum::{http, Json};
use http::StatusCode;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Serialize, Deserialize)]
pub struct RegisterError {
    status: String,
    reason: String,
    details: String,
}

#[derive(Error, Debug)]
pub enum APIError {
    #[error("Database connection error")]
    DbConnectionError(#[from] deadpool_diesel::PoolError),

    #[error("Database interaction error: {0}")]
    DbInteractionError(String),

    #[error("Unexpected error: {0}")]
    UnexpectedError(String),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("License error: {0}")]
    LicenseError(String),

    #[error("Not accessible")]
    NotAccessible(),
}

impl IntoResponse for APIError {
    fn into_response(self) -> Response {
        let error_response = match &self {
            APIError::DbConnectionError(_) => RegisterError {
                status: "failed".to_string(),
                reason: "Database Connection Error".to_string(),
                details: self.to_string(),
            },
            APIError::DbInteractionError(_) => RegisterError {
                status: "failed".to_string(),
                reason: "Database Interaction Error".to_string(),
                details: self.to_string(),
            },
            APIError::UnexpectedError(_) => RegisterError {
                status: "failed".to_string(),
                reason: "Internal Server Error".to_string(),
                details: self.to_string(),
            },
            APIError::ValidationError(_) => RegisterError {
                status: "failed".to_string(),
                reason: "Database Connection Error".to_string(),
                details: self.to_string(),
            },
            APIError::LicenseError(address) => RegisterError {
                status: "failed".to_string(),
                reason: "no_license".to_string(),
                details: format!("Address: {} does not contain the license.", address),
            },
            APIError::NotAccessible() => RegisterError {
                status: "failed".to_string(),
                reason: "not_accessible".to_string(),
                details: "The Blockfrost instance is not publically accessible.".to_string(),
            },
        };

        (StatusCode::INTERNAL_SERVER_ERROR, Json(error_response)).into_response()
    }
}
