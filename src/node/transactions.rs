use super::connection::NodeClient;
use crate::BlockfrostError;
use pallas_crypto::hash::Hasher;
use pallas_hardano::display::haskell_error::as_node_submit_error;
use pallas_network::miniprotocols::{
    localstate,
    localtxsubmission::{EraTx, Response},
};
use tracing::info;

impl NodeClient {
    /// Submits a transaction to the connected Cardano node.
    /// This API meant to be fully compatible with cardano-submit-api.
    /// Should return HTTP 200 if the transaction was accepted by the node.
    /// If the transaction was rejected, should return HTTP 400 with a JSON body:
    /// * Swagger: <https://github.com/IntersectMBO/cardano-node/blob/6e969c6bcc0f07bd1a69f4d76b85d6fa9371a90b/cardano-submit-api/swagger.yaml#L52>
    /// * Haskell code: <https://github.com/IntersectMBO/cardano-node/blob/6e969c6bcc0f07bd1a69f4d76b85d6fa9371a90b/cardano-submit-api/src/Cardano/TxSubmit/Web.hs#L158>
    pub async fn submit_transaction(&mut self, tx: Vec<u8>) -> Result<String, BlockfrostError> {
        let txid = hex::encode(Hasher::<256>::hash_cbor(&tx));

        let current_era = self
            .with_statequery(|generic_client: &mut localstate::GenericClient| {
                Box::pin(async {
                    Ok(localstate::queries_v16::get_current_era(generic_client).await?)
                })
            })
            .await?;

        let era_tx = EraTx(current_era, tx);

        // Connect to the node
        let submission_client = self.client.as_mut().unwrap().submission();

        // Submit the transaction
        match submission_client.submit_tx(era_tx).await {
            Ok(Response::Accepted) => {
                info!("Transaction accepted by the node {}", txid);
                Ok(txid)
            },
            Ok(Response::Rejected(reason)) => {
                let haskell_display = as_node_submit_error(reason);
                info!("{}: {:?}", "TxSubmitFail", haskell_display);
                Err(BlockfrostError::custom_400(haskell_display))
            },
            Err(e) => {
                let error_message = format!("Error during transaction submission: {:?}", e);

                Err(BlockfrostError::custom_400(error_message))
            },
        }
    }
}
