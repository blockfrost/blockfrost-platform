use crate::api::ApiResult;
use axum::{
    Extension,
    extract::{Path, Query},
};
use blockfrost_openapi::models::tx_metadata_label_json_inner::TxMetadataLabelJsonInner;
use common::{
    metadata::MetadataPath,
    pagination::{Pagination, PaginationQuery},
};
use dolos::client::Dolos;

pub async fn route(
    Extension(dolos): Extension<Dolos>,
    Query(pagination_query): Query<PaginationQuery>,
    Path(matadata_path): Path<MetadataPath>,
) -> ApiResult<Vec<TxMetadataLabelJsonInner>> {
    let pagination = Pagination::from_query(pagination_query).await?;

    dolos
        .metadata_txs_labels_label(&matadata_path.label, &pagination)
        .await
}
