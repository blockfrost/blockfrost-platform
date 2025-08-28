use axum::response::{IntoResponse, Response};
use axum::{http, Json};
use http::StatusCode;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
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
    #[error("Validation error: {0}")]
    Validation(String),

    #[error("License error: {0}")]
    License(String),

    #[error("Not accessible: {ip}:{port}")]
    NotAccessible { ip: SocketAddr, port: u16 },

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
            APIError::Validation(_) => (
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
            APIError::NotAccessible { ip, port } => (
                StatusCode::FORBIDDEN,
                ApiError {
                    status: "failed".to_string(),
                    reason: "not_accessible".to_string(),
                    details: format!(
                        "The server at {}:{} is not publicly accessible.",
                        ip.ip(),
                        port
                    ),
                },
            ),
            APIError::Unauthorized() => (
                StatusCode::FORBIDDEN,
                ApiError {
                    status: "failed".to_string(),
                    reason: "unauthorized".to_string(),
                    details: "You are not authorized to access the registration.".to_string(),
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
