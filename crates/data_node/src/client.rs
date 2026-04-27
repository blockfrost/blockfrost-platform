use bf_common::{errors::AppError, json_client::JsonClient};
use reqwest::Url;
use std::time::Duration;

#[derive(Clone)]
pub struct DataNode {
    pub client: JsonClient,
}

impl DataNode {
    pub fn new(endpoint: &str, request_timeout: Duration) -> Result<Self, AppError> {
        let url = Url::parse(endpoint).map_err(|e| AppError::DataNode(e.to_string()))?;
        let client = JsonClient::new(url, request_timeout)?;

        Ok(Self { client })
    }
}
