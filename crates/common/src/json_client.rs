use crate::errors::AppError;
use crate::pagination::ApplyPagination;
use crate::pagination::Pagination;
use crate::types::ApiResult;
use axum::Json;
use reqwest::{Client, Method, Url};
use serde::de::DeserializeOwned;

#[derive(Clone)]
pub struct JsonClient {
    base_url: Url,
    client: Client,
}

impl JsonClient {
    pub fn new(base_url: Url, timeout_secs: u64) -> Result<Self, AppError> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(timeout_secs))
            .build()
            .map_err(|e| AppError::Server(format!("failed to build client: {e}")))?;

        Ok(Self { base_url, client })
    }

    pub async fn get<T>(&self, path: &str) -> ApiResult<T>
    where
        T: DeserializeOwned,
    {
        let url = self.base_url.join(path)?;
        let resp = self.client.request(Method::GET, url).send().await?;
        let body = resp.json::<T>().await?;

        Ok(Json(body))
    }

    pub async fn get_paginated<T>(&self, path: &str, pagination: &Pagination) -> ApiResult<T>
    where
        T: DeserializeOwned,
    {
        let mut url = self.base_url.join(path)?;
        url.apply_pagination(pagination);

        let resp = self.client.request(Method::GET, url).send().await?;
        let body = resp.json::<T>().await?;

        Ok(Json(body))
    }
}
