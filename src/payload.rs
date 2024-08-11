use crate::errors::APIError;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Debug, Deserialize)]
pub struct Payload {
    pub mode: String,
    pub port: i32,
    pub secret: String,
    pub reward_address: String,
}

impl Payload {
    pub fn validate(&self) -> Result<(), APIError> {
        if self.mode.is_empty() {
            return Err(APIError::ValidationError(
                "Mode cannot be empty".to_string(),
            ));
        }

        if self.port <= 0 || self.port > 65535 {
            return Err(APIError::ValidationError(
                "Port must be between 1 and 65535".to_string(),
            ));
        }

        if self.secret.len() < 8 {
            return Err(APIError::ValidationError(
                "Secret must be at least 8 characters long".to_string(),
            ));
        }

        if !self.reward_address.starts_with("0x") || self.reward_address.len() != 42 {
            return Err(APIError::ValidationError(
                "Invalid reward address format".to_string(),
            ));
        }

        Ok(())
    }
}
