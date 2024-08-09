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

impl RegisterError {
    pub fn db_error() -> Self {
        Self {
            error: "Not Found".to_string(),
            message: "The requested component has not been found.".to_string(),
            status_code: 404,
        }
    }
}

impl IntoResponse for RegisterError {
    fn into_response(self) -> Response {
        let status_code = match self.status_code {
            400 => StatusCode::BAD_REQUEST,
            404 => StatusCode::NOT_FOUND,
            405 => StatusCode::METHOD_NOT_ALLOWED,
            500 => StatusCode::INTERNAL_SERVER_ERROR,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };

        let error_response = Self {
            error: self.error,
            message: self.message,
            status_code: self.status_code,
        };

        (status_code, Json(error_response)).into_response()
    }
}
