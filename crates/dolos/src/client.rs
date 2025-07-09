use common::{errors::AppError, json_client::JsonClient};
use reqwest::Url;

#[derive(Clone)]
pub struct Dolos {
    pub client: JsonClient,
}

pub struct DolosConfig {
    pub base_url: String,
    pub request_timeout: u64,
}

impl Dolos {
    pub fn new(cfg: &DolosConfig) -> Result<Self, AppError> {
        let url = Url::parse(&cfg.base_url).map_err(|e| AppError::Dolos(e.to_string()))?;
        let client = JsonClient::new(url, cfg.request_timeout)?;

        Ok(Self { client })
    }
}
