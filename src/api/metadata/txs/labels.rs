use crate::{
    BlockfrostError,
    pagination::{Pagination, PaginationQuery},
};
use axum::{Json, extract::Query};
use blockfrost_openapi::models::tx_metadata_labels_inner::TxMetadataLabelsInner;

pub async fn route(
    Query(pagination_query): Query<PaginationQuery>,
) -> Result<Json<Vec<TxMetadataLabelsInner>>, BlockfrostError> {
    let _ = Pagination::from_query(pagination_query).await?;

    Err(BlockfrostError::not_found())
}
