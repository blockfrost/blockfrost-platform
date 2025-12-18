use anyhow::{Result, anyhow};
use bf_common::errors::{AppError, BlockfrostError};
use std::{path::PathBuf, sync::Arc};
use tokio::sync::{Mutex, mpsc};
use tracing::{error, info, warn};

mod verifications;

/// Runs a `hydra-node` and sets up an L2 network with the Gateway for microtransactions.
///
/// You can safely clone it, and the clone will represent the same `hydra-node` etc.
#[derive(Clone)]
pub struct HydraManager {
    event_tx: mpsc::Sender<Event>,
}

impl HydraManager {
    pub async fn spawn(
        config: bf_common::config::HydraConfig,
        reward_address: String,
        health_errors: Arc<Mutex<Vec<BlockfrostError>>>,
    ) -> Result<Self, AppError> {
        let event_tx = State::spawn(config, reward_address, health_errors).await?;
        Ok(Self { event_tx })
    }

    pub async fn send_some_event(&self, some_value: u64) {
        self.event_tx
            .send(Event::SomeEvent {
                _some_value: some_value,
            })
            .await
            .expect("we never close the event receiver");
    }
}

enum Event {
    Restart,
    SomeEvent { _some_value: u64 },
}

struct State {
    config: bf_common::config::HydraConfig,
    _reward_address: String,
    _health_errors: Arc<Mutex<Vec<BlockfrostError>>>,
    hydra_node_exe: String,
    cardano_cli_exe: String,
    config_dir: PathBuf,
    event_tx: mpsc::Sender<Event>,
}

impl State {
    const RESTART_DELAY: std::time::Duration = std::time::Duration::from_secs(5);
    const MIN_FUEL_LOVELACE: u64 = 15_000_000;

    async fn spawn(
        config: bf_common::config::HydraConfig,
        reward_address: String,
        health_errors: Arc<Mutex<Vec<BlockfrostError>>>,
    ) -> Result<mpsc::Sender<Event>, AppError> {
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

        let (event_tx, mut event_rx) = mpsc::channel::<Event>(32);
        let self_ = Self {
            config,
            _reward_address: reward_address,
            _health_errors: health_errors,
            hydra_node_exe,
            cardano_cli_exe,
            config_dir,
            event_tx: event_tx.clone(),
        };

        self_.send(Event::Restart).await;

        tokio::spawn(async move {
            while let Some(event) = event_rx.recv().await {
                match self_.process_event(event).await {
                    Ok(()) => (),
                    Err(err) => {
                        error!(
                            "hydra-manager: error: {}; will restart in {:?}…",
                            err,
                            Self::RESTART_DELAY
                        );
                        tokio::time::sleep(Self::RESTART_DELAY).await;
                        self_.send(Event::Restart).await;
                    },
                }
            }
        });

        Ok(event_tx)
    }

    async fn send(&self, event: Event) {
        self.event_tx
            .send(event)
            .await
            .expect("we never close the event receiver");
    }

    async fn process_event(&self, event: Event) -> Result<()> {
        match event {
            Event::Restart => {
                info!("hydra-manager: starting…");

                let potential_fuel =
                    self.lovelace_on_payment_skey(&self.config.cardano_signing_key)?;
                if potential_fuel < Self::MIN_FUEL_LOVELACE {
                    Err(anyhow!(
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

                // FIXME: remove this and continue normally:
                warn!(
                    "hydra-manager: finished unexpectedly, but without an error; will restart in {:?}…",
                    Self::RESTART_DELAY
                );
                tokio::time::sleep(Self::RESTART_DELAY).await;
                self.send(Event::Restart).await;
            },
            Event::SomeEvent { .. } => todo!(),
        }
        Ok(())
    }
}
