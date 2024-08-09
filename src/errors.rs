use axum::response::{IntoResponse, Response};
use axum::{http, Json};
use http::StatusCode;
use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RegisterError {
    pub status_code: u16,
    pub error: String,
    pub message: String,
}
use thiserror::Error;

#[derive(Error, Debug)]
pub enum APIError {
    #[error("Database connection error")]
    DbConnectionError(#[from] deadpool_diesel::PoolError),

    #[error("Database interaction error: {0}")]
    DbInteractionError(String),

    #[error("Unexpected error: {0}")]
    UnexpectedError(String),
}

impl IntoResponse for APIError {
    fn into_response(self) -> Response {
        let error_response = match &self {
            APIError::DbConnectionError(_) => RegisterError {
                status_code: 500,
                error: "Database Connection Error".to_string(),
                message: self.to_string(),
            },
            APIError::DbInteractionError(_) => RegisterError {
                status_code: 500,
                error: "Database Interaction Error".to_string(),
                message: self.to_string(),
            },
            APIError::UnexpectedError(_) => RegisterError {
                status_code: 500,
                error: "Internal Server Error".to_string(),
                message: self.to_string(),
            },
        };

        (StatusCode::INTERNAL_SERVER_ERROR, Json(error_response)).into_response()
    }
}
