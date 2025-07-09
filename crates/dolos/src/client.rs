use axum::Json;
use common::errors::{AppError, BlockfrostError};
use reqwest::{Client, Method, Url};
use serde::de::DeserializeOwned;

#[derive(Clone)]
pub struct Dolos {
    base_url: Url,
    client: Client,
}

pub struct DolosConfig {
    pub base_url: String,
    pub request_timeout: u64,
}

impl Dolos {
    pub fn new(config: &DolosConfig) -> Result<Self, AppError> {
        let url = Url::parse(&config.base_url).map_err(|e| AppError::Dolos(e.to_string()))?;
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(config.request_timeout))
            .build()
            .map_err(|e| AppError::Dolos(e.to_string()))?;

        Ok(Self {
            base_url: url,
            client,
        })
    }

    pub async fn call_endpoint<T>(&self, path: &str) -> Result<Json<T>, BlockfrostError>
    where
        T: DeserializeOwned,
    {
        let url = self.base_url.join(path)?;
        let resp = self.client.request(Method::GET, url).send().await?;
        let body = resp.json::<T>().await?;

        Ok(Json(body))
    }
}
