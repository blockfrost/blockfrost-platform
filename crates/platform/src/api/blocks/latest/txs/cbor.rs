use crate::{api::ApiResult, server::state::AppState};
use axum::extract::{Query, State};
use bf_api_provider::types::BlocksTxsCborResponse;
use bf_common::pagination::{Pagination, PaginationQuery};

pub async fn route(
    State(state): State<AppState>,
    Query(pagination_query): Query<PaginationQuery>,
) -> ApiResult<BlocksTxsCborResponse> {
    let pagination = Pagination::from_query(pagination_query)?;
    let data_node = state.data_node()?;

    data_node.blocks().latest_txs_cbor(&pagination).await
}
