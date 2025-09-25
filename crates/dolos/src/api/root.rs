use crate::client::Dolos;
use common::types::ApiResult;
use serde::{Deserialize, Serialize};

pub struct DolosRoot<'a> {
    pub(crate) inner: &'a Dolos,
}

impl Dolos {
    pub fn root(&self) -> DolosRoot<'_> {
        DolosRoot { inner: self }
    }
}

#[derive(Serialize, Deserialize)]
pub struct RootResponse {
    pub url: String,
    pub version: String,
    pub revision: String,
}

impl RootResponse {
    pub fn new(url: String, version: String, revision: String) -> Self {
        RootResponse {
            url,
            version,
            revision,
        }
    }
}

impl DolosRoot<'_> {
    pub async fn get(&self) -> ApiResult<RootResponse> {
        self.inner.client.get("", None).await
    }
}
