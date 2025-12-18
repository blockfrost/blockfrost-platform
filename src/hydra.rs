use bf_common::errors::{AppError, BlockfrostError};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{error, info, warn};

pub mod verifications;

pub struct HydraManager {
    config: bf_common::config::HydraConfig,
    health_errors: Arc<Mutex<Vec<BlockfrostError>>>,
    hydra_node_exe: String,
    cardano_cli_exe: String,
}

impl HydraManager {
    const RESTART_DELAY: std::time::Duration = std::time::Duration::from_secs(5);
    const MIN_FUEL_LOVELACE: u64 = 15_000_000;

    pub fn new(
        config: bf_common::config::HydraConfig,
        health_errors: Arc<Mutex<Vec<BlockfrostError>>>,
    ) -> Result<Self, AppError> {
        let hydra_node_exe =
            bf_common::find_libexec::find_libexec("hydra-node", "HYDRA_NODE_PATH", &["--version"])
                .map_err(AppError::Server)?;
        let cardano_cli_exe =
            bf_common::find_libexec::find_libexec("cardano-cli", "CARDANO_CLI_PATH", &["version"])
                .map_err(AppError::Server)?;

        Ok(Self {
            config,
            health_errors,
            hydra_node_exe,
            cardano_cli_exe,
        })
    }

    /// Runs a `hydra-node` and sets up an L2 network with the Gateway for microtransactions.
    pub async fn run(self) {
        tokio::spawn(async move {
            loop {
                info!("hydra-manager: starting…");
                match self.run_once().await {
                    Ok(()) => warn!(
                        "hydra-manager: finished unexpectedly, but without an error; will restart in {:?}…",
                        Self::RESTART_DELAY
                    ),
                    Err(err) => error!(
                        "hydra-manager: error: {}; will restart in {:?}…",
                        err,
                        Self::RESTART_DELAY
                    ),
                }
                tokio::time::sleep(Self::RESTART_DELAY).await;
            }
        });
    }

    async fn run_once(&self) -> Result<(), Box<dyn std::error::Error>> {
        let potential_fuel = verifications::lovelace_on_payment_skey(
            &self.cardano_cli_exe,
            &self.config.cardano_signing_key,
        )?;
        if potential_fuel < Self::MIN_FUEL_LOVELACE {
            Err(format!(
                "{} ADA is too little for the Hydra L1 fees on the enterprise address associated with {:?}. Please provide at least {} ADA",
                potential_fuel as f64 / 1_000_000.0,
                self.config.cardano_signing_key,
                Self::MIN_FUEL_LOVELACE as f64 / 1_000_000.0,
            ))?
        }

        // TODO: hydra-node gen-hydra-key --output-file credentials/"$participant"-node/hydra

        info!(
            "lovelace on {:?} is {:?}",
            self.config.cardano_signing_key, potential_fuel
        );

        info!(
            "would start: {} with parameters: {:?} · {}",
            self.hydra_node_exe, self.config, self.cardano_cli_exe
        );

        Ok(())
    }
}
