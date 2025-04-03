use crate::errors::APIError;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Debug, Deserialize)]
pub struct Payload {
    pub mode: String,
    pub port: i32,
    pub secret: String,
    pub reward_address: String,
    pub api_prefix: Uuid,
}

impl Payload {
    pub fn validate(&self) -> Result<(), APIError> {
        // Validate mode
        if self.mode.is_empty() {
            return Err(APIError::Validation("Mode cannot be empty".to_string()));
        }

        if !["compact", "light", "full"].contains(&self.mode.as_str()) {
            return Err(APIError::Validation(
                "Mode must be one of 'compact', 'light', or 'full'".to_string(),
            ));
        }

        // Validate port
        if self.port <= 0 || self.port > 65535 {
            return Err(APIError::Validation(
                "Port must be between 1 and 65535".to_string(),
            ));
        }

        // Validate secret
        if self.secret.len() < 8 {
            return Err(APIError::Validation(
                "Secret must be at least 8 characters long".to_string(),
            ));
        }

        // Validate reward_address
        if self.reward_address.is_empty() {
            return Err(APIError::Validation("reward_address is empty".to_string()));
        }

        Ok(())
    }
}
