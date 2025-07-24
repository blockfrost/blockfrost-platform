use std::time::Duration;

use common::{config::DolosConfig, errors::AppError, json_client::JsonClient};
use reqwest::Url;

#[derive(Clone)]
pub struct Dolos {
    pub client: JsonClient,
}

impl Dolos {
    pub fn new(config: &Option<DolosConfig>) -> Result<Self, AppError> {
        if let Some(cfg) = config {
            let url = Url::parse(&cfg.endpoint).map_err(|e| AppError::Dolos(e.to_string()))?;
            let client = JsonClient::new(url, cfg.request_timeout, false)?;

            Ok(Dolos { client })
        } else {
            let is_disabled = true;
            let url = Url::parse("http://localhost").unwrap();
            let client = JsonClient::new(url, Duration::from_secs(0), is_disabled)?;

            Ok(Dolos { client })
        }
    }
}
