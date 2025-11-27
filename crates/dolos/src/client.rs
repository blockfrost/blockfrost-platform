use bf_common::{config::DolosConfig, errors::AppError, json_client::JsonClient};
use reqwest::Url;

#[derive(Clone)]
pub struct Dolos {
    pub client: JsonClient,
}

impl Dolos {
    pub fn new(config: Option<&DolosConfig>) -> Result<Option<Self>, AppError> {
        if let Some(cfg) = config {
            if let Some(endpoint) = &cfg.endpoint {
                let url = Url::parse(endpoint).map_err(|e| AppError::Dolos(e.to_string()))?;
                let client = JsonClient::new(url, cfg.request_timeout)?;
                let dolos = Dolos { client };

                return Ok(Some(dolos));
            }
        }

        Ok(None)
    }
}
