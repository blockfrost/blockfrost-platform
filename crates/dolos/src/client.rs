use common::{config::DolosConfig, errors::AppError, json_client::JsonClient};
use reqwest::Url;

#[derive(Clone)]
pub struct Dolos {
    pub client: JsonClient,
}

impl Dolos {
    pub fn new(cfg: &Option<DolosConfig>) -> Result<Option<Self>, AppError> {
        // no dolos configuration
        let Some(cfg) = cfg else {
            return Ok(None);
        };

        // no dolos endpoint
        let Some(endpoint) = cfg.endpoint.as_deref() else {
            return Ok(None);
        };

        let url = Url::parse(endpoint).map_err(|e| AppError::Dolos(e.to_string()))?;
        let client = JsonClient::new(url, cfg.request_timeout)?;

        Ok(Some(Dolos { client }))
    }
}
