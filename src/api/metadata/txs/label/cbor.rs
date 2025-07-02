use crate::{BlockfrostError, api::ApiResult};
use axum::extract::Query;
use blockfrost_openapi::models::tx_metadata_label_cbor_inner::TxMetadataLabelCborInner;
use common::pagination::{Pagination, PaginationQuery};

pub async fn route(
    Query(pagination_query): Query<PaginationQuery>,
) -> ApiResult<Vec<TxMetadataLabelCborInner>> {
    let _ = Pagination::from_query(pagination_query).await?;

    Err(BlockfrostError::not_found())
}
