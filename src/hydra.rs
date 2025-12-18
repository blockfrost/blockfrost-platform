use bf_common::errors::{AppError, BlockfrostError};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;

pub struct HydraManager {
    config: bf_common::config::HydraConfig,
    health_errors: Arc<Mutex<Vec<BlockfrostError>>>,
}

impl HydraManager {
    pub fn new(
        config: bf_common::config::HydraConfig,
        health_errors: Arc<Mutex<Vec<BlockfrostError>>>,
    ) -> Self {
        Self {
            config,
            health_errors,
        }
    }

    /// Runs a `hydra-node` and sets up an L2 network with the Gateway for microtransactions.
    pub async fn run(self) -> Result<(), AppError> {
        info!("Hydra::run called");

        let hydra_node_exe =
            bf_common::find_libexec::find_libexec("hydra-node", "HYDRA_NODE_PATH", &["--version"])
                .map_err(AppError::Server)?;
        let cardano_cli_exe =
            bf_common::find_libexec::find_libexec("cardano-cli", "CARDANO_CLI_PATH", &["version"])
                .map_err(AppError::Server)?;

        tokio::spawn(async move {
            loop {
                info!(
                    "would start: {} with parameters: {:?} · {}",
                    hydra_node_exe, self.config, cardano_cli_exe
                );
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            }
        });

        Ok(())
    }
}
