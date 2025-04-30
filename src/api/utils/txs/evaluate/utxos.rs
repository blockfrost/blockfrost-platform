use crate::{BlockfrostError, api::ApiResult};
use serde_json::Value;

pub async fn route() -> ApiResult<Value> {
    Err(BlockfrostError::not_found())
}
