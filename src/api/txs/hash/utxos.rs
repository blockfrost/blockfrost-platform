use crate::{BlockfrostError, api::ApiResult};
use blockfrost_openapi::models::tx_content_utxo_inputs_inner::TxContentUtxoInputsInner;

pub async fn route() -> ApiResult<TxContentUtxoInputsInner> {
    Err(BlockfrostError::not_found())
}
