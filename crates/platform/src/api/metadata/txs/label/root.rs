use crate::{api::ApiResult, server::state::AppState};
use axum::extract::{Path, Query, State};
use bf_api_provider::types::MetadataLabelJsonResponse;
use bf_common::{
    metadata::MetadataPath,
    pagination::{Pagination, PaginationQuery},
};

pub async fn route(
    State(state): State<AppState>,
    Query(pagination_query): Query<PaginationQuery>,
    Path(matadata_path): Path<MetadataPath>,
) -> ApiResult<MetadataLabelJsonResponse> {
    let pagination = Pagination::from_query(pagination_query)?;
    let data_node = state.data_node()?;

    data_node
        .metadata()
        .label_json(&matadata_path.label, &pagination)
        .await
}
