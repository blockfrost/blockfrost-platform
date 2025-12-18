use bf_common::errors::{AppError, BlockfrostError};
use std::error::Error;
use std::process::Command;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
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
        let potential_fuel = verifications::lovelace_on_payment_skey(
            &self.cardano_cli_exe,
            &self.config.cardano_signing_key,
        )?;
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

    /// Generates Hydra keys if they don’t exist.
    async fn gen_hydra_keys(&self) -> Result<(), Box<dyn Error>> {
        std::fs::create_dir_all(&self.config_dir)?;

        let key_path = self.config_dir.join("hydra.sk");

        if !key_path.exists() {
            info!("hydra-manager: generating hydra keys");

            let status = Command::new(&self.hydra_node_exe)
                .arg("gen-hydra-key")
                .arg("--output-file")
                .arg(&self.config_dir.join("hydra"))
                .status()?;

            if !status.success() {
                Err(format!("gen-hydra-key failed with status: {status}"))?;
            }
        } else {
            info!("hydra-manager: hydra keys already exist");
        }

        Ok(())
    }

    /// Generates Hydra `protocol-parameters.json` if they don’t exist. These
    /// are L1 parameters with zeroed transaction fees.
    async fn gen_protocol_parameters(&self) -> Result<(), Box<dyn Error>> {
        use serde_json::Value;

        std::fs::create_dir_all(&self.config_dir)?;

        let output = Command::new("cardano-cli")
            .args(["query", "protocol-parameters"])
            .output()?;

        if !output.status.success() {
            Err(format!("cardano-cli failed with status: {}", output.status))?;
        }

        let mut params: Value = serde_json::from_slice(&output.stdout)?;

        // .txFeeFixed := 0
        // .txFeePerByte := 0
        if let Some(obj) = params.as_object_mut() {
            obj.insert("txFeeFixed".to_string(), 0.into());
            obj.insert("txFeePerByte".to_string(), 0.into());

            // .executionUnitPrices.priceMemory := 0
            // .executionUnitPrices.priceSteps := 0
            if let Some(exec_prices) = obj
                .get_mut("executionUnitPrices")
                .and_then(Value::as_object_mut)
            {
                exec_prices.insert("priceMemory".to_string(), 0.into());
                exec_prices.insert("priceSteps".to_string(), 0.into());
            }
        }

        let pp_path = self.config_dir.join("protocol-parameters.json");
        if Self::write_json_if_changed(pp_path, &params)? {
            info!("hydra-manager: protocol parameters updated");
        } else {
            info!("hydra-manager: protocol parameters unchanged");
        }

        Ok(())
    }

    /// Writes `json` to `path` (pretty-printed) **only if** the JSON content differs
    /// from what is already on disk. Returns `true` if the file was written.
    fn write_json_if_changed<P: AsRef<Path>>(
        path: P,
        json: &serde_json::Value,
    ) -> Result<bool, Box<dyn Error>> {
        use std::fs::File;
        use std::io::Write;

        let path = path.as_ref();

        if path.exists() {
            if let Ok(existing_str) = std::fs::read_to_string(path) {
                if let Ok(existing_json) = serde_json::from_str::<serde_json::Value>(&existing_str)
                {
                    if existing_json == *json {
                        return Ok(false);
                    }
                }
            }
        }

        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }

        let mut file = File::create(path)?;
        serde_json::to_writer_pretty(&mut file, json)?;
        file.write_all(b"\n")?;

        Ok(true)
    }
}
