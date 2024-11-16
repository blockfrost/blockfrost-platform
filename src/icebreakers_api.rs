use std::sync::Arc;

use crate::{
    cli::{Config, Network},
    errors::AppError,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::{info, warn};

#[derive(Debug)]
pub struct IcebreakersAPI {
    client: Client,
    base_url: String,
    secret: String,
    mode: String,
    port: u16,
    reward_address: String,
    pub api_prefix: String,
}

#[derive(Deserialize)]
struct ErrorResponse {
    reason: String,
    details: String,
}

#[derive(Deserialize, Serialize)]
pub struct SuccessResponse {
    route: String,
}

impl IcebreakersAPI {
    /// Creates a new `IcebreakersAPI` instance or logs a warning if not configured
    pub async fn new(config: &Config) -> Result<Option<Arc<Self>>, AppError> {
        let api_url = match config.network {
            Network::Preprod | Network::Preview => "https://api-dev.icebreakers.blockfrost.io",
            Network::Mainnet => "https://icebreakers-api.blockfrost.io",
        };

        match &config.icebreakers_config {
            Some(icebreakers_config) => {
                info!("Connecting to Icebreakers API...");

                let client = Client::new();
                let base_url = api_url.to_string();

                let mut icebreakers_api = IcebreakersAPI {
                    client,
                    base_url,
                    secret: icebreakers_config.secret.clone(),
                    mode: config.mode.to_string(),
                    port: config.server_port,
                    reward_address: icebreakers_config.reward_address.clone(),
                    api_prefix: "/".to_string(),
                };

                let result = icebreakers_api.register().await?;

                // Pass the route to the icebreakers_api instance
                icebreakers_api.api_prefix = result.route;

                let icebreakers_api = Arc::new(icebreakers_api);

                info!("Successfully registered with Icebreakers API.");

                Ok(Some(icebreakers_api))
            }
            None => {
                // Logging the solitary mode warning
                warn!(" __________________________________________ ");
                warn!("/ Running in solitary mode.                \\");
                warn!("|                                          |");
                warn!("\\ You're not part of the Blockfrost fleet! /");
                warn!(" ------------------------------------------ ");
                warn!("        \\   ^__^");
                warn!("         \\  (oo)\\_______");
                warn!("            (__)\\       )\\/\\");
                warn!("                ||----w |");
                warn!("                ||     ||");

                Ok(None)
            }
        }
    }

    /// Registers with the Icebreakers API
    pub async fn register(&self) -> Result<SuccessResponse, AppError> {
        info!("Registering with icebreakers api...");

        let url = format!("{}/register", self.base_url);
        let body = json!({
            "secret": self.secret,
            "mode": self.mode,
            "port": self.port,
            "reward_address": self.reward_address,
        });

        let response = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| AppError::Registration(format!("Registering failed: {}", e)))?;

        if response.status().is_success() {
            let success_response = response.json::<SuccessResponse>().await.map_err(|e| {
                AppError::Registration(format!("Failed to parse success response: {}", e))
            })?;

            Ok(success_response)
        } else {
            let error_response = response.json::<ErrorResponse>().await.map_err(|e| {
                AppError::Registration(format!("Failed to parse error response: {}", e))
            })?;

            Err(AppError::Registration(format!(
                "Failed to register with Icebreakers API: {} details: {}",
                error_response.reason, error_response.details
            )))
        }
    }
}
