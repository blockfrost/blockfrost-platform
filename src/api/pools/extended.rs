use crate::{api::ApiResult, server::state::AppState};
use api_provider::types::PoolsListExtendedResponse;
use axum::extract::{Query, State};
use common::pagination::{Pagination, PaginationQuery};

pub async fn route(
    State(state): State<AppState>,
    Query(pagination_query): Query<PaginationQuery>,
) -> ApiResult<PoolsListExtendedResponse> {
    let dolos = state.get_dolos()?;
    let pagination = Pagination::from_query(pagination_query)?;

    dolos.pools().extended(&pagination).await
}
