use crate::client::Dolos;
use blockfrost_openapi::models::{
    tx_metadata_label_cbor_inner::TxMetadataLabelCborInner,
    tx_metadata_label_json_inner::TxMetadataLabelJsonInner,
    tx_metadata_labels_inner::TxMetadataLabelsInner,
};
use common::types::ApiResult;

impl Dolos {
    pub async fn metadata_txs_labels(&self) -> ApiResult<Vec<TxMetadataLabelsInner>> {
        self.client.get("metadata/txs/labels").await
    }

    pub async fn metadata_txs_labels_label(
        &self,
        label: &str,
    ) -> ApiResult<Vec<TxMetadataLabelJsonInner>> {
        let path = format!("metadata/txs/labels/{label}");
        self.client.get(&path).await
    }

    pub async fn metadata_txs_labels_label_cbor(
        &self,
        label: &str,
    ) -> ApiResult<Vec<TxMetadataLabelCborInner>> {
        let path = format!("metadata/txs/labels/{label}/cbor");
        self.client.get(&path).await
    }
}
