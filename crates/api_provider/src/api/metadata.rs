use crate::types::{MetadataLabelCborResponse, MetadataLabelJsonResponse, MetadataLabelsResponse};
use async_trait::async_trait;
use common::{errors::BlockfrostError, pagination::Pagination, types::ApiResult};

#[async_trait]
pub trait MetadataApi: Send + Sync + 'static {
    async fn metadata_txs_labels(&self) -> ApiResult<MetadataLabelsResponse> {
        Err(BlockfrostError::not_found())
    }

    async fn metadata_txs_labels_label(
        &self,
        _label: &str,
        _pagination: &Pagination,
    ) -> ApiResult<MetadataLabelJsonResponse> {
        Err(BlockfrostError::not_found())
    }

    async fn metadata_txs_labels_label_cbor(
        &self,
        _label: &str,
        _pagination: &Pagination,
    ) -> ApiResult<MetadataLabelCborResponse> {
        Err(BlockfrostError::not_found())
    }
}
