use super::connection::NodeClient;
use crate::{
    cbor::haskell_types::{TxSubmitFail, TxValidationError},
    BlockfrostError,
};
use pallas_crypto::hash::Hasher;
use pallas_network::miniprotocols::{
    localstate,
    localtxsubmission::{EraTx, Response},
};
use tracing::{info, warn};

impl NodeClient {
    /// Submits a transaction to the connected Cardano node.
    /// This API meant to be fully compatible with cardano-submit-api.
    /// Should return HTTP 200 if the transaction was accepted by the node.
    /// If the transaction was rejected, should return HTTP 400 with a JSON body:
    /// * Swagger: <https://github.com/IntersectMBO/cardano-node/blob/6e969c6bcc0f07bd1a69f4d76b85d6fa9371a90b/cardano-submit-api/swagger.yaml#L52>
    /// * Haskell code: <https://github.com/IntersectMBO/cardano-node/blob/6e969c6bcc0f07bd1a69f4d76b85d6fa9371a90b/cardano-submit-api/src/Cardano/TxSubmit/Web.hs#L158>
    pub async fn submit_transaction(&mut self, tx: &[u8]) -> Result<String, BlockfrostError> {
        let tx = Self::binary_or_hex_heuristic(tx);
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
            }
            Ok(Response::Rejected(reason)) => {
                // The [2..] is a Pallas bug, cf. <https://github.com/txpipe/pallas/pull/548>.
                let reason = &reason.0[2..];

                match self.fallback_decoder.decode(reason).await {
                    Ok(submit_api_json) => {
                        let error_message = "TxSubmitFail".to_string();
                        warn!(
                            "{}: {} ~ {:?}",
                            error_message,
                            hex::encode(reason),
                            submit_api_json
                        );

                        Err(BlockfrostError::custom_400(submit_api_json.to_string()))
                    }

                    Err(e) => {
                        warn!("Failed to decode error reason: {:?}", e);

                        Err(BlockfrostError::custom_400(format!(
                            "Failed to decode error reason: {:?}",
                            e
                        )))
                    }
                }
            }
            Err(e) => {
                let error_message = format!("Error during transaction submission: {:?}", e);

                Err(BlockfrostError::custom_400(error_message))
            }
        }
    }

    /// This function allows us to take both hex-encoded and raw bytes. It has
    /// to be a heuristic: if there are input bytes that are not `[0-9a-f]`,
    /// then it must be a binary string. Otherwise, we assume it’s hex encoded.
    ///
    /// **Note**: there is a small probability that the user gave us a binary
    /// string that only _looked_ like a hex-encoded one, but it’s rare enough
    /// to ignore it.
    pub fn binary_or_hex_heuristic(xs: &[u8]) -> Vec<u8> {
        let even_length = xs.len() % 2 == 0;
        let contains_non_hex = xs.iter().any(|&x| !x.is_ascii_hexdigit());
        if !even_length || contains_non_hex {
            xs.to_vec()
        } else {
            hex::decode(xs).expect("can't happen")
        }
    }

    /// Mimicks the data structure of the error response from the cardano-submit-api
    fn _unused_i_i_i_i_i_i_i_generate_error_response(error: TxValidationError) -> TxSubmitFail {
        use crate::cbor::haskell_types::{
            TxCmdError::TxCmdTxSubmitValidationError, TxSubmitFail::TxSubmitFail,
            TxValidationErrorInCardanoMode::TxValidationErrorInCardanoMode,
        };

        TxSubmitFail(TxCmdTxSubmitValidationError(
            TxValidationErrorInCardanoMode(error),
        ))
    }
}

#[cfg(test)]
mod tests {
    use crate::cbor::haskell_types::{
        ApplyConwayTxPredError::*, ApplyTxErr, ShelleyBasedEra::*, TxValidationError::*,
    };

    use super::*;

    #[test]
    fn test_generate_error_response_with_multiple_errors() {
        let validation_error = ShelleyTxValidationError {
            error: ApplyTxErr(vec![
                MempoolFailure("error1".to_string()),
                MempoolFailure("error2".to_string()),
            ]),
            era: ShelleyBasedEraConway,
        };

        let error_string = serde_json::to_string(
            &NodeClient::_unused_i_i_i_i_i_i_i_generate_error_response(validation_error),
        )
        .expect("Failed to convert error to JSON");
        let expected_error_string = r#"{"tag":"TxSubmitFail","contents":{"tag":"TxCmdTxSubmitValidationError","contents":{"tag":"TxValidationErrorInCardanoMode","contents":{"kind":"ShelleyTxValidationError","error":["MempoolFailure (error1)","MempoolFailure (error2)"],"era":"ShelleyBasedEraConway"}}}}"#;

        assert_eq!(error_string, expected_error_string);
    }

    #[test]
    fn test_binary_or_hex_heuristic() {
        let hex_string = "84a300d90102818258203ac521101f8d";
        assert_eq!(
            NodeClient::binary_or_hex_heuristic(hex_string.as_bytes()),
            NodeClient::binary_or_hex_heuristic(
                &hex::decode("84a300d90102818258203ac521101f8d").unwrap()
            )
        )
    }
}
