use crate::{api::ApiResult, server::state::AppState};
use axum::extract::{Query, State};
use bf_api_provider::types::PoolsListExtendedResponse;
use bf_common::pagination::{Pagination, PaginationQuery};

pub async fn route(
    State(state): State<AppState>,
    Query(pagination_query): Query<PaginationQuery>,
) -> ApiResult<PoolsListExtendedResponse> {
    let data_node = state.data_node()?;
    let pagination = Pagination::from_query(pagination_query)?;

    data_node.pools().extended(&pagination).await
}
