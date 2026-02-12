use crate::client::DataNode;
use bf_api_provider::types::{
    MetadataLabelCborResponse, MetadataLabelJsonResponse, MetadataLabelsResponse,
};
use bf_common::{pagination::Pagination, types::ApiResult};

pub struct DataNodeMetadata<'a> {
    pub(crate) inner: &'a DataNode,
}

impl DataNode {
    pub fn metadata(&self) -> DataNodeMetadata<'_> {
        DataNodeMetadata { inner: self }
    }
}

impl DataNodeMetadata<'_> {
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
