use std::time::Duration;

use crate::errors::AppError;
use crate::errors::BlockfrostError;
use crate::pagination::ApplyPagination;
use crate::pagination::Pagination;
use crate::types::ApiResult;

use axum::Json;
use reqwest::{Client, Method, Url};
use serde::de::DeserializeOwned;
use tracing::{error, info};

#[derive(Clone)]
pub struct JsonClient {
    base_url: Url,
    client: Client,
}

impl JsonClient {
    pub fn new(base_url: Url, timeout_secs: Duration) -> Result<Self, AppError> {
        let client = Client::builder()
            .timeout(timeout_secs)
            .build()
            .map_err(|e| AppError::Server(format!("failed to build client: {e}")))?;

        Ok(Self { base_url, client })
    }

    pub async fn get<T>(&self, path: &str, pagination: Option<&Pagination>) -> ApiResult<T>
    where
        T: DeserializeOwned,
    {
        let mut url = self.base_url.join(path)?;

        if let Some(pag) = pagination {
            url.apply_pagination(pag);
        }

        let url_str = url.to_string();
        let resp = self.client.request(Method::GET, url).send().await?;

        info!(path, url = %url_str, ?pagination, "JsonClient GET");

        if resp.status() == 404 {
            return Err(BlockfrostError::not_found());
        }

        let status = resp.status();
        let body_text = resp.text().await?;

        let body: T = serde_json::from_str(&body_text).map_err(|e| {
            error!(
                path,
                url = %url_str,
                status = %status,
                response_body = %body_text,
                error = %e,
                "JsonClient failed to parse response"
            );
            e
        })?;

        Ok(Json(body))
    }
}
