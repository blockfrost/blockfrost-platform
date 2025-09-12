use crate::{api::ApiResult, server::state::AppState};
use api_provider::types::MetadataLabelsResponse;
use axum::extract::{Query, State};
use common::pagination::{Pagination, PaginationQuery};

pub async fn route(
    State(state): State<AppState>,
    Query(pagination_query): Query<PaginationQuery>,
) -> ApiResult<MetadataLabelsResponse> {
    let pagination = Pagination::from_query(pagination_query)?;
    let dolos = state.get_dolos()?;

    dolos.metadata().labels(&pagination).await
}
