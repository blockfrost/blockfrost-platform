use bf_common::{config::DataNodeConfig, errors::AppError, json_client::JsonClient};
use reqwest::Url;

#[derive(Clone)]
pub struct Dolos {
    pub client: JsonClient,
}

impl Dolos {
    pub fn new(config: &DataNodeConfig) -> Result<Self, AppError> {
        let url = Url::parse(&config.endpoint).map_err(|e| AppError::Dolos(e.to_string()))?;
        let client = JsonClient::new(url, config.request_timeout)?;

        Ok(Dolos { client })
    }
}
