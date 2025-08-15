use crate::{api::ApiResult, server::state::AppState};
use api_provider::types::BlocksResponse;
use axum::extract::{Path, Query, State};
use common::{
    blocks::BlocksPath,
    pagination::{Pagination, PaginationQuery},
};

pub async fn route(
    State(state): State<AppState>,
    Query(pagination_query): Query<PaginationQuery>,
    Path(blocks_path): Path<BlocksPath>,
) -> ApiResult<BlocksResponse> {
    let pagination = Pagination::from_query(pagination_query).await?;

    state
        .api
        .dolos
        .blocks_previous(&blocks_path.hash_or_number, &pagination)
        .await
}
