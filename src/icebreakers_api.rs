use crate::{
    config::Config, config::Network, errors::AppError, load_balancer::LoadBalancerConfig,
    server::ApiPrefix,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use tracing::{info, warn};

#[derive(Debug)]
pub struct IcebreakersAPI {
    client: Client,
    base_url: String,
    secret: String,
    mode: String,
    port: u16,
    reward_address: String,
    api_prefix: ApiPrefix,
}

#[derive(Deserialize)]
struct ErrorResponse {
    reason: String,
    details: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct SuccessResponse {
    pub route: String,
    /// Experimental: a list of WebSocket URIs and access tokens that the
    /// `blockfrost-platform` should connect to. Blockfrost.io request and
    /// responses, as well as network reconfiguration requests (in the future)
    /// will be will be passed to the `blockfrost-platform` over the socket(s),
    /// eventually eliminating the need for each relay to expose a public
    /// routable port, and making network configuration on their side much
    /// easier. We keep the previous setup and backwards compatibility, and just
    /// observe this experiment.
    ///
    /// It has to temporarily be an option, to keep compatibility with the older
    /// IceBreakers API.
    pub load_balancers: Option<Vec<LoadBalancerConfig>>,
}

impl IcebreakersAPI {
    /// Creates a new `IcebreakersAPI` instance or logs a warning if not configured
    pub async fn new(
        config: &Config,
        api_prefix: ApiPrefix,
    ) -> Result<Option<Arc<Self>>, AppError> {
        let api_url = match config.network {
            Network::Preprod | Network::Preview => "https://api-dev.icebreakers.blockfrost.io",
            Network::Mainnet | Network::Custom => "https://icebreakers-api.blockfrost.io",
        };

        match &config.icebreakers_config {
            Some(icebreakers_config) => {
                let client = Client::builder()
                    .local_address(config.server_address)
                    .build()
                    .map_err(|e| AppError::Registration(format!("Registering failed: {}", e)))?;
                let base_url = api_url.to_string();
                let icebreakers_api = IcebreakersAPI {
                    client,
                    base_url,
                    secret: icebreakers_config.secret.clone(),
                    mode: config.mode.to_string(),
                    port: config.server_port,
                    reward_address: icebreakers_config.reward_address.clone(),
                    api_prefix,
                };

                let icebreakers_api = Arc::new(icebreakers_api);

                Ok(Some(icebreakers_api))
            },
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
            },
        }
    }

    /// Registers with the Icebreakers API
    pub async fn register(&self) -> Result<SuccessResponse, AppError> {
        info!("Connecting to Icebreakers API...");
        info!("Registering with icebreakers api...");

        let url = format!("{}/register", self.base_url);
        let body = json!({
            "secret": self.secret,
            "mode": self.mode,
            "port": self.port,
            "reward_address": self.reward_address,
            "api_prefix": self.api_prefix.0.unwrap_or_default(),
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

            info!("Successfully registered with Icebreakers API.");

            // In case we get a URI without a protocol (http: or https: or ws: or wss:):
            let fallback_proto = if self.base_url.starts_with("https:") {
                "wss:"
            } else {
                "ws:"
            };

            let success_response = SuccessResponse {
                load_balancers: Some(
                    success_response
                        .load_balancers
                        .into_iter()
                        .flatten()
                        .map(|lb| {
                            if lb.uri.starts_with("//") {
                                info!(
                                    "load balancer: falling back to {} for a schemeless load balancer URI: {}",
                                    fallback_proto,
                                    lb.uri
                                );
                                LoadBalancerConfig {
                                    uri: format!("{}{}", fallback_proto, lb.uri),
                                    ..lb
                                }
                            } else {
                                lb
                            }
                        })
                        .collect(),
                ),
                ..success_response
            };

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
