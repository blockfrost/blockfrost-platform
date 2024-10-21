use std::process::Command;

use crate::errors::{AppError, BlockfrostError};
use axum::body::Bytes;
use pallas_crypto::hash::Hasher;
use pallas_network::{
    facades::NodeClient,
    miniprotocols::localtxsubmission::{EraTx, Response},
};
use tracing::{info, warn};

pub struct Node {
    client: NodeClient,
    cardano_bin_path: String,
}

impl Node {
    /// Creates a new `Node` instance
    pub async fn new(
        socket: &str,
        network_magic: u64,
        cardano_bin_path: &str,
    ) -> Result<Node, AppError> {
        info!("Connecting to node socket {} ...", socket);

        let client = NodeClient::connect(socket, network_magic).await?;

        info!("Connection to node was successfully established.");
        Ok(Node {
            client,
            cardano_bin_path: cardano_bin_path.to_owned(),
        })
    }

    /// Submits a transaction to the connected Cardano node.
    pub async fn submit_transaction(&mut self, tx: Bytes) -> Result<String, BlockfrostError> {
        info!("Submitting transaction to node.");

        let tx_vec = tx.to_vec();
        let txid = hex::encode(Hasher::<256>::hash_cbor(&tx_vec));
        let era_tx = EraTx(6, tx_vec);

        match self.client.submission().submit_tx(era_tx).await? {
            Response::Accepted => Ok(txid),
            Response::Rejected(reason) => {
                warn!("Transaction was rejected: {}", hex::encode(&reason.0));

                Err(BlockfrostError::custom_400(format!(
                    "Transaction was rejected: {}",
                    hex::encode(&reason.0)
                )))
            }
        }
    }

    // Gets the node version from the connected Cardano node.
    pub async fn version(&mut self) -> Result<String, BlockfrostError> {
        Self::get_node_version_from_bin(&self.cardano_bin_path)
    }

    fn get_node_version_from_bin(node_bin_path: &str) -> Result<String, BlockfrostError> {
        info!("Getting version of the node from {}", node_bin_path);

        let version_output = Command::new(node_bin_path).arg("--version").output();

        match version_output {
            Ok(output) => {
                if output.status.success() {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    Ok(stdout.replace('\n', " ").trim().to_string())
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    Err(BlockfrostError::internal_server_error(
                        stderr.replace('\n', " ").trim().to_string(),
                    ))
                }
            }
            Err(e) => Err(BlockfrostError::internal_server_error(format!(
                "Failed to execute command: {} {}",
                node_bin_path, e
            ))),
        }
    }
}
