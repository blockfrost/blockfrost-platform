use crate::{BlockfrostError, api::ApiResult, server::state::AppState};
use axum::extract::{Path, Query, State};
use bf_api_provider::types::BlocksAddressesContentResponse;
use bf_common::blocks::{BlockData, BlocksPath};
use bf_common::pagination::PaginationQuery;

pub async fn route(
    State(_state): State<AppState>,
    Query(_pagination_query): Query<PaginationQuery>,
    Path(blocks_path): Path<BlocksPath>,
) -> ApiResult<BlocksAddressesContentResponse> {
    let _ = BlockData::from_string(blocks_path.hash_or_number)?;

    Err(BlockfrostError::not_found())
}
