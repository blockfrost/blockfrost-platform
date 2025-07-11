use crate::client::Dolos;
use blockfrost_openapi::models::{
    tx_metadata_label_cbor_inner::TxMetadataLabelCborInner,
    tx_metadata_label_json_inner::TxMetadataLabelJsonInner,
    tx_metadata_labels_inner::TxMetadataLabelsInner,
};
use common::{pagination::Pagination, types::ApiResult};

impl Dolos {
    pub async fn metadata_txs_labels(&self) -> ApiResult<Vec<TxMetadataLabelsInner>> {
        self.client.get("metadata/txs/labels").await
    }

    pub async fn metadata_txs_labels_label(
        &self,
        label: &str,
        pagination: &Pagination,
    ) -> ApiResult<Vec<TxMetadataLabelJsonInner>> {
        let path = format!("metadata/txs/labels/{label}");

        self.client.get_paginated(&path, pagination).await
    }

    pub async fn metadata_txs_labels_label_cbor(
        &self,
        label: &str,
        pagination: &Pagination,
    ) -> ApiResult<Vec<TxMetadataLabelCborInner>> {
        let path = format!("metadata/txs/labels/{label}/cbor");

        self.client.get_paginated(&path, pagination).await
    }
}
