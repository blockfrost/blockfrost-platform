use std::time::Duration;

use crate::errors::AppError;
use crate::errors::BlockfrostError;
use crate::pagination::ApplyPagination;
use crate::pagination::Pagination;
use crate::types::ApiResult;
use axum::Json;
use reqwest::{Client, Method, Url};
use serde::de::DeserializeOwned;
use tracing::info;

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

        let body = resp.json::<T>().await?;

        Ok(Json(body))
    }
}
