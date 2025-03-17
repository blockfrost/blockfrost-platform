use super::connection::NodeClient;
use crate::BlockfrostError;
use pallas_crypto::hash::Hasher;
use pallas_hardano::display::haskell_error::{as_cbor_decode_failure, as_node_submit_error};
use pallas_network::miniprotocols::{
    localstate,
    localtxsubmission::{EraTx, Response},
};
use pallas_primitives::conway::Tx;
use tracing::{debug, info};

impl NodeClient {
    /// Submits a transaction to the connected Cardano node.
    /// This API meant to be fully compatible with cardano-submit-api.
    /// Should return HTTP 200 if the transaction was accepted by the node.
    /// If the transaction was rejected, should return HTTP 400 with a JSON body:
    /// * Swagger: <https://github.com/IntersectMBO/cardano-node/blob/6e969c6bcc0f07bd1a69f4d76b85d6fa9371a90b/cardano-submit-api/swagger.yaml#L52>
    /// * Haskell code: <https://github.com/IntersectMBO/cardano-node/blob/6e969c6bcc0f07bd1a69f4d76b85d6fa9371a90b/cardano-submit-api/src/Cardano/TxSubmit/Web.hs#L158>
    pub async fn submit_transaction(&mut self, tx: Vec<u8>) -> Result<String, BlockfrostError> {
        Self::assert_valid_tx_cbor(&tx)?;

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

    fn assert_valid_tx_cbor(tx: &[u8]) -> Result<(), BlockfrostError> {
        match pallas_codec::minicbor::decode::<Tx>(tx) {
            Ok(_) => Ok(()),
            Err(e) => {
                debug!("Invalid TX CBOR submitted: {:?}", e);
                Err(BlockfrostError::custom_400(as_cbor_decode_failure(
                    e.to_string(),
                    e.position().unwrap_or(0) as u64,
                )))
            },
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_assert_valid_tx_cbor() {
        // Invalid CBOR bytes
        let invalid_tx = vec![0xFF, 0xFF];
        assert!(NodeClient::assert_valid_tx_cbor(&invalid_tx).is_err());

        let invalid_tx = "aaaaaa".as_bytes().to_vec();
        assert!(NodeClient::assert_valid_tx_cbor(&invalid_tx).is_err());

        let invalid_tx = "8".as_bytes().to_vec();
        assert!(NodeClient::assert_valid_tx_cbor(&invalid_tx).is_err());

        let invalid_tx = "821".as_bytes().to_vec();
        assert!(NodeClient::assert_valid_tx_cbor(&invalid_tx).is_err());
        // Empty bytes
        let empty_tx = vec![];
        assert!(NodeClient::assert_valid_tx_cbor(&empty_tx).is_err());

        let invalid_tx = "".as_bytes().to_vec();
        assert!(NodeClient::assert_valid_tx_cbor(&invalid_tx).is_err());

        // Sample valid CBOR bytes representing a minimal TransactionBody
        // This is a very basic transaction body encoded as CBOR
        let valid_tx = hex::decode("84a300d90102818258205176274bef11d575edd6aa72392aaf993a07f736e70239c1fb22d4b1426b22bc01018282583900ddf1eb9ce2a1561e8f156991486b97873fb6969190cbc99ddcb3816621dcb03574152623414ed354d2d8f50e310f3f2e7d167cb20e5754271a003d09008258390099a5cb0fa8f19aba38cacf8a243d632149129f882df3a8e67f6bd512bcb0cde66a545e9fbc7ca4492f39bca1f4f265cc1503b4f7d6ff205c1b000000024f127a7c021a0002a2ada100d90102818258208b83e59abc9d7a66a77be5e0825525546a595174f8b929f164fcf5052d7aab7b5840709c64556c946abf267edd90b8027343d065193ef816529d8fa7aa2243f1fd2ec27036a677974199e2264cb582d01925134b9a20997d5a734da298df957eb002f5f6").unwrap();
        assert!(NodeClient::assert_valid_tx_cbor(&valid_tx).is_ok());
    }
}
