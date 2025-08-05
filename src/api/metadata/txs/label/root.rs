use crate::{api::ApiResult, server::state::AppState};
use axum::extract::{Path, Query, State};
use blockfrost_openapi::models::tx_metadata_label_json_inner::TxMetadataLabelJsonInner;
use common::{
    metadata::MetadataPath,
    pagination::{Pagination, PaginationQuery},
};

pub async fn route(
    State(state): State<AppState>,
    Query(pagination_query): Query<PaginationQuery>,
    Path(matadata_path): Path<MetadataPath>,
) -> ApiResult<Vec<TxMetadataLabelJsonInner>> {
    let pagination = Pagination::from_query(pagination_query).await?;

    state
        .api
        .dolos
        .metadata_txs_labels_label(&matadata_path.label, &pagination)
        .await
}
