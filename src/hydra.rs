use anyhow::{Result, anyhow};
use bf_common::errors::{AppError, BlockfrostError};
use std::{path::PathBuf, sync::Arc};
use tokio::sync::{Mutex, mpsc};
use tracing::{debug, error, info, warn};

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
        network: bf_common::types::Network,
        node_socket_path: String,
        reward_address: String,
        health_errors: Arc<Mutex<Vec<BlockfrostError>>>,
    ) -> Result<Self, AppError> {
        let event_tx = State::spawn(
            config,
            network,
            node_socket_path,
            reward_address,
            health_errors,
        )
        .await?;
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
    network: bf_common::types::Network,
    genesis: bf_api_provider::types::GenesisResponse,
    node_socket_path: String,
    hydra_scripts_tx_id: String,
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
    // FIXME: this should most probably be back to the default of 600 seconds:
    const CONTESTATION_PERIOD_SECONDS: std::time::Duration = std::time::Duration::from_secs(60);

    async fn spawn(
        config: bf_common::config::HydraConfig,
        network: bf_common::types::Network,
        node_socket_path: String,
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

        // FIXME: also define them in a `build.rs` script without Nix – consult
        // `flake.lock` to get the exact Hydra version.
        let hydra_scripts_tx_id: String = {
            use bf_common::types::Network::*;
            match network {
                Mainnet => env!("HYDRA_SCRIPTS_TX_ID_MAINNET").into(),
                Preprod => env!("HYDRA_SCRIPTS_TX_ID_PREPROD").into(),
                Preview => env!("HYDRA_SCRIPTS_TX_ID_PREVIEW").into(),
                Custom => Err(AppError::Server(
                    "hydra-manager: can only run on known networks (Mainnet, Preprod, Preview)"
                        .into(),
                ))?,
            }
        };

        let genesis = {
            use bf_common::genesis::*;
            genesis().by_network(&network)
        };

        let (event_tx, mut event_rx) = mpsc::channel::<Event>(32);
        let self_ = Self {
            config,
            network,
            genesis,
            node_socket_path,
            hydra_scripts_tx_id,
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

                let potential_fuel = self
                    .lovelace_on_payment_skey(&self.config.cardano_signing_key)
                    .await?;
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

                self.start_hydra_node().await?;
            },
            Event::SomeEvent { .. } => todo!(),
        }
        Ok(())
    }

    async fn start_hydra_node(&self) -> Result<()> {
        use std::process::Stdio;
        use tokio::io::{AsyncBufReadExt, BufReader};

        // FIXME: save the ports in an `Arc<Mutex<u16>` for future use
        let api_port = Self::pick_free_tcp_port().await?;
        let metrics_port = Self::pick_free_tcp_port().await?;

        // FIXME: do the `h2h` ports have to be the same on both sides? :facepalm:
        // FIXME: the ports must be proposed by the Gateway, this will be safer
        let our_h2h_port = Self::pick_free_tcp_port().await?;
        let their_h2h_port = Self::pick_free_tcp_port().await?;

        // FIXME: actually exchange them through the `blockfrost-gateway` registration
        let their_hydra_vkey = "/home/mw/.config/blockfrost-platform/hydra/tmp_their_keys/hydra.vk";
        let their_cardano_vkey =
            "/home/mw/.config/blockfrost-platform/hydra/tmp_their_keys/payment.vk";

        // FIXME: somehow do shutdown once we’re killed
        // cf. <https://github.com/IntersectMBO/cardano-node/blob/10.6.1/cardano-node/src/Cardano/Node/Handlers/Shutdown.hs#L123-L148>
        // cf. <https://input-output-rnd.slack.com/archives/C06J9HK7QCQ/p1764782397820079>
        // TODO: Write a ticket in `hydra-node`.

        let mut child = tokio::process::Command::new(&self.hydra_node_exe)
            .arg("--node-id")
            .arg("platform-node")
            .arg("--persistence-dir")
            .arg(self.config_dir.join("persistence"))
            .arg("--cardano-signing-key")
            .arg(&self.config.cardano_signing_key)
            .arg("--hydra-signing-key")
            .arg(self.config_dir.join("hydra.sk"))
            .arg("--hydra-scripts-tx-id")
            .arg(&self.hydra_scripts_tx_id)
            .arg("--ledger-protocol-parameters")
            .arg(self.config_dir.join("protocol-parameters.json"))
            .arg("--contestation-period")
            .arg(format!("{}s", Self::CONTESTATION_PERIOD_SECONDS.as_secs()))
            .args(if self.network == bf_common::types::Network::Mainnet {
                vec!["-mainnet".to_string()]
            } else {
                vec![
                    "--testnet-magic".to_string(),
                    format!("{}", self.genesis.network_magic),
                ]
            })
            .arg("--node-socket")
            .arg(&self.node_socket_path)
            .arg("--api-port")
            .arg(format!("{api_port}"))
            .arg("--api-host")
            .arg("127.0.0.1")
            .arg("--listen")
            .arg(format!("127.0.0.1:{our_h2h_port}"))
            .arg("--peer")
            .arg(format!("127.0.0.1:{their_h2h_port}"))
            .arg("--monitoring-port")
            .arg(format!("{metrics_port}"))
            .arg("--hydra-verification-key")
            .arg(their_hydra_vkey)
            .arg("--cardano-verification-key")
            .arg(their_cardano_vkey)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let stdout = child.stdout.take().expect("child stdout");
        let stderr = child.stderr.take().expect("child stderr");

        tokio::spawn(async move {
            let mut lines = BufReader::new(stdout).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                debug!("{}", line);
            }
            debug!("hydra-node: stdout closed");
        });

        tokio::spawn(async move {
            let mut lines = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                info!("{}", line);
            }
            info!("hydra-node: stderr closed");
        });

        let event_tx = self.event_tx.clone();
        tokio::spawn(async move {
            match child.wait().await {
                Ok(status) => {
                    warn!("hydra-node: exited: {}", status);
                    tokio::time::sleep(Self::RESTART_DELAY).await;
                    event_tx
                        .send(Event::Restart)
                        .await
                        .expect("we never close the event receiver");
                },
                Err(e) => {
                    error!("hydra-node: failed to wait: {e}");
                },
            }
        });

        Ok(())
    }

    /// Finds a free port by bind to port 0, to let the OS pick a free port.
    async fn pick_free_tcp_port() -> std::io::Result<u16> {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let port = listener.local_addr()?.port();
        drop(listener);
        Ok(port)
    }
}
