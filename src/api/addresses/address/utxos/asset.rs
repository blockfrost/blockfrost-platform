use crate::{BlockfrostError, api::ApiResult, server::state::AppState};
use api_provider::types::AddressesUtxosResponse;
use axum::extract::{Path, Query, State};
use common::{
    addresses::{AddressInfo, AddressesPath},
    pagination::{Pagination, PaginationQuery},
};

pub async fn route(
    Path(address_path): Path<AddressesPath>,
    State(app_state): State<AppState>,
    Query(pagination_query): Query<PaginationQuery>,
) -> ApiResult<AddressesUtxosResponse> {
    let AddressesPath { address, asset: _ } = address_path;
    let _ = Pagination::from_query(pagination_query).await?;
    let _ = AddressInfo::from_address(&address, app_state.config.network.clone())?;

    Err(BlockfrostError::not_found())
}
