use axum::{
    response::{IntoResponse, Response},
    {http, Json},
};
use http::StatusCode;
use pallas_network::miniprotocols::localtxsubmission::Error as TxSubmissionError;
use serde::{Deserialize, Serialize};
use std::env::VarError;
use std::{array::TryFromSliceError, fmt, io};
use thiserror::Error;
use tracing::error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Node connection error: {0}")]
    Node(String),

    #[error("Icebreakers registration error: {0}")]
    Registration(String),

    #[error("Server startup error: {0}")]
    Server(String),
}

/// Main error type.
/// Contains the following fields:
/// - status_code: the HTTP status code to return
/// - error: a short description of the error
/// - message: a longer description of the error
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BlockfrostError {
    pub error: String,
    pub message: String,
    pub status_code: u16,
}

impl From<std::num::TryFromIntError> for BlockfrostError {
    fn from(err: std::num::TryFromIntError) -> Self {
        Self::internal_server_error(format!("ConversionError: {}", err))
    }
}

impl From<serde_json::Error> for BlockfrostError {
    fn from(err: serde_json::Error) -> Self {
        Self::internal_server_error(format!("SerdeError: {}", err))
    }
}

impl From<AppError> for BlockfrostError {
    fn from(err: AppError) -> Self {
        match err {
            AppError::Node(e) => Self::internal_server_error(e),
            AppError::Registration(e) => Self::internal_server_error(e),
            AppError::Server(e) => Self::internal_server_error(e),
        }
    }
}

impl From<VarError> for AppError {
    fn from(err: VarError) -> Self {
        AppError::Server(err.to_string())
    }
}

impl From<io::Error> for AppError {
    fn from(err: io::Error) -> Self {
        error!("I/O Error occurred: {}", err);
        AppError::Server(err.to_string())
    }
}

impl From<pallas_network::miniprotocols::txsubmission::Error> for BlockfrostError {
    fn from(err: pallas_network::miniprotocols::txsubmission::Error) -> Self {
        BlockfrostError::internal_server_error(format!("TxSubmissionError: {}", err))
    }
}

impl From<pallas_network::miniprotocols::handshake::Error> for BlockfrostError {
    fn from(err: pallas_network::miniprotocols::handshake::Error) -> Self {
        BlockfrostError::internal_server_error(format!("CardanoNodeHandshakeError: {}", err))
    }
}

impl From<pallas_network::miniprotocols::localstate::ClientError> for BlockfrostError {
    fn from(err: pallas_network::miniprotocols::localstate::ClientError) -> Self {
        BlockfrostError::internal_server_error(format!("localstate::ClientError: {}", err))
    }
}

impl From<TryFromSliceError> for BlockfrostError {
    fn from(err: TryFromSliceError) -> Self {
        BlockfrostError::internal_server_error(format!("Hash conversion failed: {}", err))
    }
}

impl From<pallas_network::facades::Error> for AppError {
    fn from(err: pallas_network::facades::Error) -> Self {
        AppError::Node(err.to_string())
    }
}

impl From<TxSubmissionError> for BlockfrostError {
    fn from(err: TxSubmissionError) -> Self {
        BlockfrostError::internal_server_error(format!("Transaction submission error: {}", err))
    }
}

impl BlockfrostError {
    /// Our custom 404 error
    pub fn not_found() -> Self {
        Self {
            error: "Not Found".to_string(),
            message: "The requested component has not been found.".to_string(),
            status_code: 404,
        }
    }

    /// Our custom 400 error
    pub fn custom_400(message: String) -> Self {
        Self {
            error: "Bad Request".to_string(),
            message,
            status_code: 400,
        }
    }

    /// This error is converted in middleware to internal_server_error_user
    pub fn internal_server_error(error: String) -> Self {
        Self {
            error: "Internal Server Error".to_string(),
            message: error,
            status_code: 500,
        }
    }

    /// This is internal server error for user with generic message
    pub fn internal_server_error_user() -> Self {
        Self {
            error: "Internal Server Error".to_string(),
            message: "An unexpected response was received from the backend.".to_string(),
            status_code: 500,
        }
    }

    pub fn method_not_allowed() -> Self {
        Self::custom_400("Invalid path. Please check https://docs.blockfrost.io/".to_string())
    }
}

impl fmt::Display for BlockfrostError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "BlockfrostError: {}", self.message)
    }
}

impl IntoResponse for BlockfrostError {
    fn into_response(self) -> Response {
        let status_code = match self.status_code {
            400 => StatusCode::BAD_REQUEST,
            404 => StatusCode::NOT_FOUND,
            405 => StatusCode::METHOD_NOT_ALLOWED,
            500 => StatusCode::INTERNAL_SERVER_ERROR,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };

        error!("Error occurred: {} - {}", self.error, self.message);

        let error_response = self.clone();

        (status_code, Json(error_response)).into_response()
    }
}
