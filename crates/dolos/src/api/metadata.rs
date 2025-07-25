use crate::client::Dolos;
use api_provider::{
    api::metadata::MetadataApi,
    types::{MetadataLabelCborResponse, MetadataLabelJsonResponse, MetadataLabelsResponse},
};
use async_trait::async_trait;
use common::{pagination::Pagination, types::ApiResult};

#[async_trait]
impl MetadataApi for Dolos {
    async fn metadata_txs_labels(&self) -> ApiResult<MetadataLabelsResponse> {
        self.client.get("metadata/txs/labels", None).await
    }

    async fn metadata_txs_labels_label(
        &self,
        label: &str,
        pagination: &Pagination,
    ) -> ApiResult<MetadataLabelJsonResponse> {
        let path = format!("metadata/txs/labels/{label}");

        self.client.get(&path, Some(pagination)).await
    }

    async fn metadata_txs_labels_label_cbor(
        &self,
        label: &str,
        pagination: &Pagination,
    ) -> ApiResult<MetadataLabelCborResponse> {
        let path = format!("metadata/txs/labels/{label}/cbor");

        self.client.get(&path, Some(pagination)).await
    }
}
