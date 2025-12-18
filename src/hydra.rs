use bf_common::errors::{AppError, BlockfrostError};
use std::error::Error;
use std::{path::PathBuf, sync::Arc};
use tokio::sync::Mutex;
use tracing::{error, info, warn};

pub mod verifications;

pub struct HydraManager {
    config: bf_common::config::HydraConfig,
    reward_address: String,
    health_errors: Arc<Mutex<Vec<BlockfrostError>>>,
    hydra_node_exe: String,
    cardano_cli_exe: String,
    config_dir: PathBuf,
}

impl HydraManager {
    const RESTART_DELAY: std::time::Duration = std::time::Duration::from_secs(5);
    const MIN_FUEL_LOVELACE: u64 = 15_000_000;

    pub fn new(
        config: bf_common::config::HydraConfig,
        reward_address: String,
        health_errors: Arc<Mutex<Vec<BlockfrostError>>>,
    ) -> Result<Self, AppError> {
        let hydra_node_exe =
            bf_common::find_libexec::find_libexec("hydra-node", "HYDRA_NODE_PATH", &["--version"])
                .map_err(AppError::Server)?;
        let cardano_cli_exe =
            bf_common::find_libexec::find_libexec("cardano-cli", "CARDANO_CLI_PATH", &["version"])
                .map_err(AppError::Server)?;

        let config_dir = dirs::config_dir()
            .expect("Could not determine config directory")
            .join("blockfrost-platform")
            .join("hydra");

        Ok(Self {
            config,
            reward_address,
            health_errors,
            hydra_node_exe,
            cardano_cli_exe,
            config_dir,
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

    async fn run_once(&self) -> Result<(), Box<dyn Error>> {
        let potential_fuel = self.lovelace_on_payment_skey(&self.config.cardano_signing_key)?;
        if potential_fuel < Self::MIN_FUEL_LOVELACE {
            Err(format!(
                "hydra-manager: {} ADA is too little for the Hydra L1 fees on the enterprise address associated with {:?}. Please provide at least {} ADA",
                potential_fuel as f64 / 1_000_000.0,
                self.config.cardano_signing_key,
                Self::MIN_FUEL_LOVELACE as f64 / 1_000_000.0,
            ))?
        }

        info!(
            "hydra-manager: fuel on cardano_signing_key: {:?} lovelace",
            potential_fuel
        );

        self.gen_hydra_keys().await?;
        self.gen_protocol_parameters().await?;

        Ok(())
    }
}
