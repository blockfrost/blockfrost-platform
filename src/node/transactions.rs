use super::connection::NodeConn;
use crate::errors::BlockfrostError;
use pallas_crypto::hash::Hasher;
use pallas_network::miniprotocols::localtxsubmission::{EraTx, Response};
use tracing::{info, warn};

pub async fn submit_transaction(
    client: &mut NodeConn,
    tx: String,
) -> Result<String, BlockfrostError> {
    let tx = hex::decode(tx).map_err(|e| BlockfrostError::custom_400(e.to_string()))?;
    let txid = hex::encode(Hasher::<256>::hash_cbor(&tx));
    let era_tx = EraTx(6, tx);

    // Connect to the node
    let submission_client = client.underlying.as_mut().unwrap().submission();

    // Submit the transaction
    match submission_client.submit_tx(era_tx).await {
        Ok(Response::Accepted) => {
            info!("Transaction accepted by the node {}", txid);
            Ok(txid)
        }
        Ok(Response::Rejected(reason)) => {
            let reason = reason.0;
            let msg_res = client.try_decode_error(&reason);
            let error_message = format!("Transaction rejected with reason: {:?}", msg_res);

            warn!(error_message);

            Err(BlockfrostError::custom_400(error_message))
        }
        Err(e) => {
            let error_message = format!("Error during transaction submission: {:?}", e);

            Err(BlockfrostError::custom_400(error_message))
        }
    }
}
