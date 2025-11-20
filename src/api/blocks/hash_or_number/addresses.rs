use crate::{BlockfrostError, api::ApiResult, server::state::AppState};
use api_provider::types::BlocksAddressesContentResponse;
use axum::extract::{Path, Query, State};
use common::blocks::{BlockData, BlocksPath};
use common::pagination::PaginationQuery;

pub async fn route(
    State(_state): State<AppState>,
    Query(_pagination_query): Query<PaginationQuery>,
    Path(blocks_path): Path<BlocksPath>,
) -> ApiResult<BlocksAddressesContentResponse> {
    let _ = BlockData::from_string(blocks_path.hash_or_number)?;

    Err(BlockfrostError::not_found())
}
