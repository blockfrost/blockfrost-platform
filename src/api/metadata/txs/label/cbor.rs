use crate::{
    BlockfrostError,
    pagination::{Pagination, PaginationQuery},
};
use axum::{Json, extract::Query};
use blockfrost_openapi::models::tx_metadata_label_cbor_inner::TxMetadataLabelCborInner;

pub async fn route(
    Query(pagination_query): Query<PaginationQuery>,
) -> Result<Json<Vec<TxMetadataLabelCborInner>>, BlockfrostError> {
    let _ = Pagination::from_query(pagination_query).await?;

    Err(BlockfrostError::not_found())
}
