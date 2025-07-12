use common::{config::DolosConfig, errors::AppError, json_client::JsonClient};
use reqwest::Url;

#[derive(Clone)]
pub struct Dolos {
    pub client: JsonClient,
}

impl Dolos {
    pub fn new(config: &Option<DolosConfig>) -> Result<Option<Self>, AppError> {
        if let Some(cfg) = config {
            let url = Url::parse(&cfg.endpoint).map_err(|e| AppError::Dolos(e.to_string()))?;
            let client = JsonClient::new(url, cfg.request_timeout)?;

            Ok(Some(Dolos { client }))
        } else {
            // dolos endpoint not configured
            Ok(None)
        }
    }
}
