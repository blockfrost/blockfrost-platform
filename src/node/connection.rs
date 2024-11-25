use crate::{cbor::haskell_types::TxValidationError, BlockfrostError};
use pallas_codec::minicbor::{display, Decoder};
use pallas_network::{
    facades::NodeClient as NodeClientFacade, miniprotocols::localstate, multiplexer::Error,
};
use std::{boxed::Box, pin::Pin};
use tracing::warn;

/// Our wrapper around [`pallas_network::facades::NodeClient`]. If you only use
/// this, you won’t get any deadlocks, inconsistencies, etc.
pub struct NodeClient {
    /// Note: this is an [`Option`] *only* to satisfy the borrow checker. It’s
    /// *always* [`Some`]. See [`NodeConnPoolManager::recycle`] for an
    /// explanation.
    pub(in crate::node) client: Option<NodeClientFacade>,
}

impl NodeClient {
    /// We always have to release the [`localstate::GenericClient`], even on errors,
    /// otherwise `cardano-node` stalls. If you use this function, it’s handled for you.
    pub async fn with_statequery<A, F>(&mut self, action: F) -> Result<A, BlockfrostError>
    where
        F: for<'a> FnOnce(
            &'a mut localstate::GenericClient,
        ) -> Pin<
            Box<dyn std::future::Future<Output = Result<A, BlockfrostError>> + 'a + Sync + Send>,
        >,
    {
        // Acquire the client
        let client = self.client.as_mut().unwrap().statequery();
        client.acquire(None).await?;

        // Run the action and ensure the client is released afterwards
        let result = action(client).await;

        // Always release the client, even if action fails
        if let Err(e) = client.send_release().await {
            warn!("Failed to release client: {:?}", e);
        }

        result
    }

    /// Pings the node, e.g. to see if the connection is still alive.
    pub async fn ping(&mut self) -> Result<(), BlockfrostError> {
        // FIXME: we should be able to use `miniprotocols::keepalive`
        // (cardano-cli does), but for some reason it’s not added to
        // `NodeClient`? Let’s try to acquire a local state client instead:

        self.with_statequery(|_| Box::pin(async { Ok(()) })).await
    }

    pub fn try_decode_error(buffer: &[u8]) -> Result<TxValidationError, Error> {
        let maybe_error = Decoder::new(buffer[2..].as_ref()).decode();

        match maybe_error {
            Ok(error) => Ok(error),
            Err(err) => {
                let buffer_display = display(buffer);
                warn!(
                    "Failed to decode error: {:?}, buffer: {}",
                    err, buffer_display
                );

                // Decoding failures are not errors, but some missing implementation or mis-implementations on our side.
                // A decoding failure is a bug in our code, not a bug in the node.
                // It should not effect the program flow, but should be logged and reported.
                Err(Error::Decoding(err.to_string()))
            }
        }
    }
}

#[cfg(test)]
mod tests {

    use crate::cbor::haskell_types::{ApplyTxErr, ShelleyBasedEra};

    use super::*;

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
            Ok(TxValidationError::ShelleyTxValidationError {
                error: ApplyTxErr(errors),
                era,
            }) => {
                assert!(
                    era == ShelleyBasedEra::ShelleyBasedEraConway,
                    "Expected ShelleyBasedEraConway"
                );
                assert_eq!(errors.len(), error_count, "Errors count mismatch",);
            }
            Err(error) => panic!("Failed to decode cbor: {:?}, error: {:?}", cbor_hex, error),
            _ => panic!("Expected ShelleyTxValidationError"),
        }
    }
}
