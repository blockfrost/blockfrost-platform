use crate::{
    BlockfrostError,
    api::ApiResult,
    pagination::{Pagination, PaginationQuery},
};
use axum::extract::Query;
use blockfrost_openapi::models::tx_metadata_labels_inner::TxMetadataLabelsInner;

pub async fn route(
    Query(pagination_query): Query<PaginationQuery>,
) -> ApiResult<Vec<TxMetadataLabelsInner>> {
    let _ = Pagination::from_query(pagination_query).await?;

    Err(BlockfrostError::not_found())
}
