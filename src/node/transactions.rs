use super::connection::NodeClient;
use crate::BlockfrostError;
use crate::cbor::validation::validate_tx_cbor;
use pallas_crypto::hash::Hasher;
use pallas_hardano::display::haskell_error::as_node_submit_error;
use pallas_network::miniprotocols::{
    localstate,
    localtxsubmission::{EraTx, Response},
};
use tracing::{error, info};

impl NodeClient {
    /// Submits a transaction to the connected Cardano node.
    /// This API meant to be fully compatible with cardano-submit-api.
    /// Should return HTTP 200 if the transaction was accepted by the node.
    /// If the transaction was rejected, should return HTTP 400 with a JSON body:
    /// * Swagger: <https://github.com/IntersectMBO/cardano-node/blob/6e969c6bcc0f07bd1a69f4d76b85d6fa9371a90b/cardano-submit-api/swagger.yaml#L52>
    /// * Haskell code: <https://github.com/IntersectMBO/cardano-node/blob/6e969c6bcc0f07bd1a69f4d76b85d6fa9371a90b/cardano-submit-api/src/Cardano/TxSubmit/Web.hs#L158>
    pub async fn submit_transaction(&mut self, tx: Vec<u8>) -> Result<String, BlockfrostError> {
        validate_tx_cbor(&tx)?;

        let current_era = self
            .with_statequery(|generic_client: &mut localstate::GenericClient| {
                Box::pin(async {
                    Ok(localstate::queries_v16::get_current_era(generic_client).await?)
                })
            })
            .await?;

        let era_tx = EraTx(current_era, tx.clone());

        // Connect to the node
        let submission_client = self.client.as_mut().unwrap().submission();

        // Submit the transaction
        match submission_client.submit_tx(era_tx).await {
            Ok(Response::Accepted) => {
                let txid = hex::encode(Hasher::<256>::hash_cbor(&tx));

                info!(
                    "N2C[{}]: Transaction accepted by the node: {}",
                    self.connection_id, txid
                );
                Ok(txid)
            },
            Ok(Response::Rejected(reason)) => {
                let haskell_display = as_node_submit_error(reason);
                info!(
                    "N2C[{}]: {}: {}, CBOR: {}",
                    self.connection_id,
                    "TxSubmitFail",
                    haskell_display,
                    hex::encode(&tx)
                );
                Err(BlockfrostError::custom_400(haskell_display))
            },
            Err(e) => {
                let error_message = format!(
                    "Error during transaction submission: {:?}, CBOR: {}",
                    e,
                    hex::encode(&tx)
                );
                self.invalidate_connection(&error_message); // Never use this connection again.
                error!(
                    "{}: {}, CBOR: {}",
                    "TxSubmitFail",
                    error_message,
                    hex::encode(&tx)
                );

                Err(BlockfrostError::custom_400(error_message))
            },
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    /// This test makes sure in case of invalid CBOR transaction, submit_transaction returns an error without even going to the node.
    #[tokio::test]
    async fn test_submit_transaction_invalid() {
        let mut client = NodeClient {
            client: None,
            connection_id: 0,
            unrecoverable_error_happened: false,
            network_magic: 2,
        };

        // Test invalid CBOR transaction
        let invalid_tx = vec![0xFF, 0xFF];
        let result = client.submit_transaction(invalid_tx).await;
        assert!(result.is_err());

        // Test invalid CBOR transaction 2
        let invalid_tx = "84a800848258204c16d304e6d531c59afd87a9199b7bb4175bc131b3d6746917901046b662963c00825820893c3f630c0b2db16d041c388aa0d58746ccbbc44133b2d7a3127a72c79722f1018258200998adb591c872a241776e39fe855e04b2d7c361008e94c582f59b6b6ccc452c028258208380ce7240ba59187f6450911f74a70cf3d2749228badb2e7cd10fb6499355f503018482581d61e15900a9a62a8fb01f936a25bf54af209c7ed1248c4e5abd05ec4e76821a0023ba63a1581ca0028f350aaabe0545fdcb56b039bfb08e4bb4d8c4d7c3c7d481c235a145484f534b5900a300581d71cba5c6770fe7b30ebc1fa32f01938c150513211360ded23ac76e36b301821a006336d5a3581c239075b83c03c2333eacd0b0beac6b8314f11ce3dc0c047012b0cad4a144706f6f6c01581c3547b4325e495d529619335603ababde10025dceafa9ed34b1fb6611a158208b284793d3bd4967244a2ddd68410d56d06d36ac8d201429b937096a2e8234bc1b7ffffffffffade6b581ca0028f350aaabe0545fdcb56b039bfb08e4bb4d8c4d7c3c7d481c235a145484f534b59195e99028201d818583ad8799fd8799f4040ffd8799f581ca0028f350aaabe0545fdcb56b039bfb08e4bb4d8c4d7c3c7d481c23545484f534b59ff1a006336d5195e99ff825839016d06090559d8ed2988aa5b2fff265d668cf552f4f62278c0128f816c0a48432e080280d0d9b15edb65563995f97ce236035afea568e660d1821a00118f32a1581c2f8b2d1f384485896f38406173fa11df2a4ce53b4b0886138b76597aa1476261746368657201825839016d06090559d8ed2988aa5b2fff265d668cf552f4f62278c0128f816c0a48432e080280d0d9b15edb65563995f97ce236035afea568e660d11a06d9f713021a000ab9e00b582027f17979d848d6472896266dd8bf39f7251ca23798713464bc407bf637286c230d81825820cf5de9189b958f8ad64c1f1837c2fa4711d073494598467a1c1a59589393eae20310825839016d06090559d8ed2988aa5b2fff265d668cf552f4f62278c0128f816c0a48432e080280d0d9b15edb65563995f97ce236035afea568e660d11a08666c75111a001016d01282825820bf93dc59c10c19c35210c2414779d7391ca19128cc7b13794ea85af5ff835f59008258201c37df764f8261edce8678b197767668a91d544b2b203fb5d0cf9acc10366e7600a200818258200eabfa083d7969681d2fc8e825a5f79e1c40f03aeac46ecd94bf5c5790db1bc058409a029ddd3cdde65598bb712c640ea63eeebfee526ce49bd0983b4d1fdca858481ddf931bf0354552cc0a7d3365e2f03fdb457c0466cea8b371b645f9b6d0c2010582840001d8799fd8799f011a006336d5195e991b7ffffffffffade6bd8799f1a000539e7ff01ffff821a000b46e41a0a7f3ca4840003d87d80821a002dccfe1a28868be8f5f6".as_bytes().to_vec();
        let result = client.submit_transaction(invalid_tx).await;
        assert!(result.is_err());
    }
}
