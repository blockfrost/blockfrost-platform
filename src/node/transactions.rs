use super::connection::NodeClient;
use crate::{
    cbor::haskell_types::{TxSubmitFail, TxValidationError},
    BlockfrostError,
};
use pallas_codec::minicbor::Decoder;
use pallas_crypto::hash::Hasher;
use pallas_network::{
    miniprotocols::{
        localstate,
        localtxsubmission::{EraTx, Response},
    },
    multiplexer::Error,
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
            Ok(Response::Rejected(reason)) => match NodeClient::try_decode_error(&reason.0) {
                Ok(error) => {
                    let haskell_display = serde_json::to_string(&error).unwrap();
                    warn!("{}: {:?}", "TxSubmitFail", haskell_display);
                    Err(BlockfrostError::custom_400(haskell_display))
                }

                Err(e) => {
                    warn!("Failed to decode error reason: {:?}", e);

                    Err(BlockfrostError::custom_400(format!(
                        "Failed to decode error reason: {:?}",
                        e
                    )))
                }
            },
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

    pub fn try_decode_error(buffer: &[u8]) -> Result<TxSubmitFail, Error> {
        let maybe_error = Decoder::new(buffer).decode();

        match maybe_error {
            Ok(error) => Ok(NodeClient::wrap_error_response(error)),
            Err(err) => {
                warn!(
                    "Failed to decode error: {:?}, buffer: {}",
                    err,
                    hex::encode(buffer)
                );

                // Decoding failures are not errors, but some missing implementation or mis-implementations on our side.
                // A decoding failure is a bug in our code, not a bug in the node.
                // It should not effect the program flow, but should be logged and reported.
                Err(Error::Decoding(err.to_string()))
            }
        }
    }
    /// Mimicks the data structure of the error response from the cardano-submit-api
    pub fn wrap_error_response(
        error: TxValidationError,
    ) -> crate::cbor::haskell_types::TxSubmitFail {
        use crate::cbor::haskell_types::{
            TxCmdError::TxCmdTxSubmitValidationError, TxSubmitFail,
            TxValidationErrorInCardanoMode::TxValidationErrorInCardanoMode,
        };

        TxSubmitFail::TxSubmitFail(TxCmdTxSubmitValidationError(
            TxValidationErrorInCardanoMode(error),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
