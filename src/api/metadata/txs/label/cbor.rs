use crate::{api::ApiResult, server::state::AppState};
use api_provider::types::MetadataLabelCborResponse;
use axum::extract::{Path, Query, State};
use common::{
    metadata::MetadataPath,
    pagination::{Pagination, PaginationQuery},
};

pub async fn route(
    State(state): State<AppState>,
    Query(pagination_query): Query<PaginationQuery>,
    Path(matadata_path): Path<MetadataPath>,
) -> ApiResult<MetadataLabelCborResponse> {
    let pagination = Pagination::from_query(pagination_query)?;
    let dolos = state.get_dolos()?;

    dolos
        .metadata()
        .label_cbor(&matadata_path.label, &pagination)
        .await
}
