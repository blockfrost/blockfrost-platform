use super::connection::NodeConn;
use crate::{cbor::haskell_types::TxValidationError, errors::BlockfrostError};
use pallas_crypto::hash::Hasher;
use pallas_network::miniprotocols::localtxsubmission::{EraTx, Response};
use tracing::{info, warn};

impl NodeConn {
    /// Submits a transaction to the connected Cardano node.
    pub async fn submit_transaction(&mut self, tx: String) -> Result<String, BlockfrostError> {
        let tx = hex::decode(tx).map_err(|e| BlockfrostError::custom_400(e.to_string()))?;
        let txid = hex::encode(Hasher::<256>::hash_cbor(&tx));
        let era_tx = EraTx(6, tx);

        // Connect to the node
        let submission_client = self.underlying.as_mut().unwrap().submission();

        // Submit the transaction
        match submission_client.submit_tx(era_tx).await {
            Ok(Response::Accepted) => {
                info!("Transaction accepted by the node {}", txid);
                Ok(txid)
            }
            Ok(Response::Rejected(reason)) => {
                let reason = reason.0;

                let msg_res = Self::try_decode_error(&reason);

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

    /// Mimicks the data structure of the error response from the cardano-submit-api
    fn generate_error_response(error: TxValidationError) {
        use crate::cbor::haskell_types::{
            TxCmdError::TxCmdTxSubmitValidationError, TxSubmitFail::TxSubmitFail,
            TxValidationErrorInCardanoMode::TxValidationErrorInCardanoMode,
        };
 
         use blockfrost_platform_macros::{define_paren_format, ParenFormat};

        define_paren_format!();

        let wrapped_error = TxSubmitFail(TxCmdTxSubmitValidationError(TxValidationErrorInCardanoMode(error)));

        let json_error =
            serde_json::to_string(&wrapped_error).expect("Failed to convert error to JSON");
        println!("{}", json_error);
    }
}

#[cfg(test)]
mod tests {
    use crate::cbor::haskell_types::{ApplyConwayTxPredError::*, ApplyTxErr, ShelleyBasedEra::*, TxValidationError::*};

    use super::*;

    #[test]
    fn test_generate_error_response() {
        let validation_error = ShelleyTxValidationError{
            error: ApplyTxErr(vec![MempoolFailure(
                "testing".to_string(),
            )]),
            era: ShelleyBasedEraConway,
        };
       
        NodeConn::generate_error_response(validation_error);
    }
}
