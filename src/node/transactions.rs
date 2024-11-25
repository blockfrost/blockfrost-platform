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
    /// Should return Http 200 if the transaction was accepted by the node.
    /// If the transaction was rejected, should return Http 400 with a JSON body.
    /// swagger: https://github.com/IntersectMBO/cardano-node/blob/6e969c6bcc0f07bd1a69f4d76b85d6fa9371a90b/cardano-submit-api/swagger.yaml#L52
    /// Haskell code:  https://github.com/IntersectMBO/cardano-node/blob/6e969c6bcc0f07bd1a69f4d76b85d6fa9371a90b/cardano-submit-api/src/Cardano/TxSubmit/Web.hs#L158
    pub async fn submit_transaction(&mut self, tx: String) -> Result<String, BlockfrostError> {
        let tx = hex::decode(tx).map_err(|e| BlockfrostError::custom_400(e.to_string()))?;
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
                let reason = reason.0;
                let msg_res = Self::try_decode_error(&reason);

                match msg_res {
                    Ok(decoded_error) => {
                        let error_response = Self::generate_error_response(decoded_error);
                        let error_message = serde_json::to_string(&error_response)
                            .unwrap_or_else(|_| "Failed to serialize error response".to_string());

                        warn!(
                            "reason in cbor: {}, error message: {}",
                            hex::encode(reason),
                            error_message
                        );

                        Err(BlockfrostError::custom_400(error_message))
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

    pub fn try_decode_error(buffer: &[u8]) -> Result<TxValidationError, Error> {
        let maybe_error = Decoder::new(buffer).decode();

        match maybe_error {
            Ok(error) => Ok(error),
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
    fn generate_error_response(error: TxValidationError) -> TxSubmitFail {
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
    use serde::Deserialize;

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

        let error_string =
            serde_json::to_string(&NodeClient::generate_error_response(validation_error))
                .expect("Failed to convert error to JSON");
        let expected_error_string = r#"{"tag":"TxSubmitFail","contents":{"tag":"TxCmdTxSubmitValidationError","contents":{"tag":"TxValidationErrorInCardanoMode","contents":{"kind":"ShelleyTxValidationError","error":["MempoolFailure (error1)","MempoolFailure (error2)"],"era":"ShelleyBasedEraConway"}}}}"#;

        assert_eq!(error_string, expected_error_string);
    }

    #[test]
    fn test_try_decode_error() {
        assert_decoding(
            "8202818206828201820083061b00000002362a77301b0000000253b9c11d8201820083051a00028bfd18ad", 2
        );
        assert_decoding("8202818206828201820083051a000151351a00074b8582076162", 2);
    }
    fn assert_decoding(cbor_hex: &str, error_count: usize) {
        let buffer = hex::decode(cbor_hex).unwrap();

        let error = NodeClient::try_decode_error(&buffer);

        match error {
            Ok(ShelleyTxValidationError {
                error: ApplyTxErr(errors),
                era,
            }) => {
                assert!(
                    era == ShelleyBasedEraConway,
                    "Expected ShelleyBasedEraConway"
                );
                assert_eq!(errors.len(), error_count, "Errors count mismatch",);
            }
            Err(error) => panic!("Failed to decode cbor: {:?}, error: {:?}", cbor_hex, error),
            _ => panic!("Expected ShelleyTxValidationError"),
        }
    }

    #[test]
    fn test_decoding_with_cases() {
        let cases = read_cases_from_file();
        for case in cases.test_cases {
            let buffer = hex::decode(&case.cbor).unwrap();
            let error = NodeClient::try_decode_error(&buffer);

            match error {
                Ok(ShelleyTxValidationError {
                    error: ApplyTxErr(errors),
                    era: _,
                }) => {
                    assert_eq!(errors.len(), case.json_naive.len(), "Errors count mismatch",);

                    // Serialize each error to JSON
                    let mut generated_errors_json: Vec<String> = errors
                        .iter()
                        .map(|e| {
                            let err = serde_json::to_string(e).expect("Failed to serialize error");
                            err[1..err.len() - 1].to_string() // we remove the leading and trailing quotes since serde adds them
                        })
                        .collect();

                    // Clone and sort the expected errors from the test case
                    let mut expected_errors_json = case.json_naive.clone();

                    // Sort both vectors since the order might differ
                    generated_errors_json.sort();
                    expected_errors_json.sort();

                    // Compare the sorted lists of errors
                    assert_eq!(
                        generated_errors_json, expected_errors_json,
                        "Errors do not match"
                    );
                }
                Err(error) => panic!(
                    "Failed to decode cbor in the case: {:?}, error: {:?}",
                    case, error
                ),
                _ => panic!("Expected ShelleyTxValidationError, case: {:?}", case),
            }
        }
    }

    #[derive(Deserialize, Debug)]
    #[serde(rename_all(deserialize = "camelCase"))]
    pub struct CborTestCases {
        _seed: u64,
        _type_tag: String,
        test_cases: Vec<CborTestCase>,
    }

    #[derive(Deserialize, Debug)]
    #[serde(rename_all(deserialize = "camelCase"))]
    /// A struct representing a CBOR test case.
    ///
    /// # Fields
    ///
    /// * `cbor` - A string representing the CBOR-encoded error reasons.
    /// * `haskell_repr` - A string representation of the transaction in Haskell format. This field contains the transaction error as a Haskell string.
    /// * `json_naive` - A vector of strings representing the errors in JSON format.
    /// * `json_submit_api` - A string representing the error JSON format for the cardano_submit_api.
    pub struct CborTestCase {
        cbor: String,
        _haskell_repr: String,
        json_naive: Vec<String>,
        #[serde(rename(deserialize = "jsonSubmitAPI"))]
        _json_submit_api: String,
    }

    /// Reads CBOR test cases from a JSON file located at `tests/fixtures/cbor/cases.json`.
    ///
    /// The file path is constructed using the `CARGO_MANIFEST_DIR` environment variable to ensure
    /// it is relative to the project's root directory.
    ///
    /// # Returns
    ///
    /// * `CborTestCases` - A struct containing the parsed test cases from the JSON file.
    ///
    /// # Panics
    ///
    /// This function will panic if:
    /// * The file cannot be opened.
    /// * The JSON content cannot be parsed.
    ///
    fn read_cases_from_file() -> CborTestCases {
        let file_path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/cbor/cases.json"
        );

        let file = std::fs::File::open(file_path)
            .unwrap_or_else(|_| panic!("Failed to open file: {}", file_path));

        let reader = std::io::BufReader::new(file);

        serde_json::from_reader(reader).expect("Failed to parse JSON")
    }
}
