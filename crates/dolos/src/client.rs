use axum::Json;
use blockfrost_openapi::models::{block_content::BlockContent, tx_content::TxContent};
use common::{
    errors::AppError,
    pagination::{ApplyPagination, Pagination},
    types::ApiResult,
};
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

    async fn call_json<T>(&self, path: &str) -> ApiResult<T>
    where
        T: DeserializeOwned,
    {
        let url = self.base_url.join(path)?;

        let resp = self.client.request(Method::GET, url).send().await?;
        let body = resp.json::<T>().await?;

        Ok(Json(body))
    }

    async fn call_json_paginated<T>(&self, path: &str, pagination: &Pagination) -> ApiResult<T>
    where
        T: DeserializeOwned,
    {
        let mut url = self.base_url.join(path)?;
        url.apply_pagination(pagination);

        let resp = self.client.request(Method::GET, url).send().await?;
        let body = resp.json::<T>().await?;

        Ok(Json(body))
    }

    pub async fn blocks_latest(&self) -> ApiResult<BlockContent> {
        self.call_json("blocks/latest").await
    }

    pub async fn blocks_latest_txs(&self) -> ApiResult<Vec<TxContent>> {
        self.call_json("blocks/latest/txs").await
    }

    pub async fn blocks_hash_or_number(&self, hash_or_number: &str) -> ApiResult<BlockContent> {
        let path = &format!("blocks/{hash_or_number}");

        self.call_json(path).await
    }

    pub async fn blocks_previous(
        &self,
        hash_or_number: &str,
        pagination: Pagination,
    ) -> ApiResult<Vec<BlockContent>> {
        let path = &format!("blocks/{hash_or_number}/previous");

        self.call_json_paginated(path, &pagination).await
    }

    pub async fn blocks_next(
        &self,
        hash_or_number: &str,
        pagination: Pagination,
    ) -> ApiResult<Vec<BlockContent>> {
        let path = &format!("blocks/{hash_or_number}/next");

        self.call_json_paginated(path, &pagination).await
    }
}
