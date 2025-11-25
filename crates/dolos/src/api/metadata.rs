use crate::client::Dolos;
use bf_api_provider::types::{
    MetadataLabelCborResponse, MetadataLabelJsonResponse, MetadataLabelsResponse,
};
use bf_common::{pagination::Pagination, types::ApiResult};

pub struct DolosMetadata<'a> {
    pub(crate) inner: &'a Dolos,
}

impl Dolos {
    pub fn metadata(&self) -> DolosMetadata<'_> {
        DolosMetadata { inner: self }
    }
}

impl DolosMetadata<'_> {
    pub async fn labels(&self, pagination: &Pagination) -> ApiResult<MetadataLabelsResponse> {
        self.inner
            .client
            .get("metadata/txs/labels", Some(pagination))
            .await
    }

    pub async fn label_json(
        &self,
        label: &str,
        pagination: &Pagination,
    ) -> ApiResult<MetadataLabelJsonResponse> {
        let path = format!("metadata/txs/labels/{label}");

        self.inner.client.get(&path, Some(pagination)).await
    }

    pub async fn label_cbor(
        &self,
        label: &str,
        pagination: &Pagination,
    ) -> ApiResult<MetadataLabelCborResponse> {
        let path = format!("metadata/txs/labels/{label}/cbor");

        self.inner.client.get(&path, Some(pagination)).await
    }
}
