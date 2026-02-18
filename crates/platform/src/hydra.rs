use anyhow::{Result, anyhow};
use bf_common::errors::{AppError, BlockfrostError};
use std::time::Duration;
use std::{path::PathBuf, sync::Arc};
use tokio::sync::{Mutex, mpsc};
use tracing::{debug, error, info, warn};

pub mod tunnel;
pub mod tunnel2;
pub mod verifications;

/// Runs a `hydra-node` and sets up an L2 network with the Gateway for microtransactions.
///
/// You can safely clone it, and the clone will represent the same `hydra-node` etc.
#[derive(Clone)]
pub struct HydraController {
    event_tx: mpsc::Sender<Event>,
}

#[derive(serde::Deserialize, serde::Serialize, Debug, PartialEq, Eq, Clone)]
pub struct KeyExchangeRequest {
    pub machine_id: String,
    pub platform_cardano_vkey: serde_json::Value,
    pub platform_hydra_vkey: serde_json::Value,
    pub accepted_platform_h2h_port: Option<u16>,
}

#[derive(serde::Deserialize, serde::Serialize, Debug, PartialEq, Eq, Clone)]
pub struct KeyExchangeResponse {
    pub machine_id: String,
    pub gateway_cardano_vkey: serde_json::Value,
    pub gateway_hydra_vkey: serde_json::Value,
    pub hydra_scripts_tx_id: String,
    pub protocol_parameters: serde_json::Value,
    pub contestation_period: std::time::Duration,
    /// Unfortunately the ports have to be the same on both sides, so
    /// since we’re tunneling through the WebSocket, and our hosts are
    /// both 127.0.0.1, the Gateway has to propose the port on the
    /// Platform, too (as both sides open both ports).
    pub proposed_platform_h2h_port: u16,
    pub gateway_h2h_port: u16,
    /// This being set to `true` means that the ceremony is successful, and the
    /// Gateway is going to start its own `hydra-node`, and the Platform should too.
    pub kex_done: bool,
}

pub struct TerminateRequest;

impl HydraController {
    // FIXME: refactor
    #[allow(clippy::too_many_arguments)]
    pub async fn spawn(
        config: bf_common::config::HydraConfig,
        network: bf_common::types::Network,
        node_socket_path: String,
        reward_address: String,
        health_errors: Arc<Mutex<Vec<BlockfrostError>>>,
        kex_requests: mpsc::Sender<KeyExchangeRequest>,
        kex_responses: mpsc::Receiver<KeyExchangeResponse>,
        terminate_reqs: mpsc::Receiver<TerminateRequest>,
    ) -> Result<Self, AppError> {
        let event_tx = State::spawn(
            config,
            network,
            node_socket_path,
            reward_address,
            health_errors,
            kex_requests,
            kex_responses,
            terminate_reqs,
        )
        .await
        .map_err(|e| AppError::Server(format!("{e}")))?;
        Ok(Self { event_tx })
    }

    pub async fn terminate(&self) {
        let _ = self.event_tx.send(Event::Terminate).await;
    }
}

enum Event {
    Restart,
    Terminate,
    KeyExchangeResponse(KeyExchangeResponse),
    TryToCommit,
    MonitorStates,
}

// FIXME: don’t construct all key and other paths manually, keep them in a single place
struct State {
    config: bf_common::config::HydraConfig,
    network: bf_common::types::Network,
    genesis: bf_api_provider::types::GenesisResponse,
    node_socket_path: String,
    platform_cardano_vkey: serde_json::Value,
    _reward_address: String,
    _health_errors: Arc<Mutex<Vec<BlockfrostError>>>,
    kex_requests: mpsc::Sender<KeyExchangeRequest>,
    api_port: u16,
    hydra_node_exe: String,
    cardano_cli_exe: String,
    config_dir: PathBuf,
    event_tx: mpsc::Sender<Event>,
    last_hydra_head_state: String,
    hydra_pid: Option<u32>,
}

impl State {
    const RESTART_DELAY: std::time::Duration = std::time::Duration::from_secs(5);
    const MIN_FUEL_LOVELACE: u64 = 15_000_000;

    // FIXME: refactor
    #[allow(clippy::too_many_arguments)]
    async fn spawn(
        config: bf_common::config::HydraConfig,
        network: bf_common::types::Network,
        node_socket_path: String,
        reward_address: String,
        health_errors: Arc<Mutex<Vec<BlockfrostError>>>,
        kex_requests: mpsc::Sender<KeyExchangeRequest>,
        kex_responses: mpsc::Receiver<KeyExchangeResponse>,
        terminate_reqs: mpsc::Receiver<TerminateRequest>,
    ) -> Result<mpsc::Sender<Event>> {
        let hydra_node_exe =
            bf_common::find_libexec::find_libexec("hydra-node", "HYDRA_NODE_PATH", &["--version"])
                .map_err(|e| anyhow!(e))?;
        let cardano_cli_exe =
            bf_common::find_libexec::find_libexec("cardano-cli", "CARDANO_CLI_PATH", &["version"])
                .map_err(|e| anyhow!(e))?;

        // FIXME: config dir prob. needs to be gateway specific? Test it!
        let gateway_prefix = "_default";

        let config_dir = dirs::config_dir()
            .expect("Could not determine config directory")
            .join("blockfrost-platform")
            .join("hydra")
            .join(network.as_str())
            .join(gateway_prefix);

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
            platform_cardano_vkey: serde_json::Value::Null,
            _reward_address: reward_address,
            _health_errors: health_errors,
            kex_requests,
            api_port: 0,
            hydra_node_exe,
            cardano_cli_exe,
            config_dir,
            event_tx: event_tx.clone(),
            last_hydra_head_state: String::new(),
            hydra_pid: None,
        };

        let platform_cardano_vkey = self_
            .derive_vkey_from_skey(&self_.config.cardano_signing_key)
            .await?;
        let mut self_ = Self {
            platform_cardano_vkey,
            ..self_
        };

        self_.send(Event::Restart).await;

        let event_tx_ = event_tx.clone();
        tokio::spawn(async move {
            let mut kex_responses = kex_responses;
            while let Some(resp) = kex_responses.recv().await {
                event_tx_
                    .send(Event::KeyExchangeResponse(resp))
                    .await
                    .expect("we never close the event receiver");
            }
        });

        let event_tx_ = event_tx.clone();
        tokio::spawn(async move {
            let mut terminate_reqs = terminate_reqs;
            while terminate_reqs.recv().await.is_some() {
                event_tx_
                    .send(Event::Terminate)
                    .await
                    .expect("we never close the event receiver");
            }
        });

        tokio::spawn(async move {
            while let Some(event) = event_rx.recv().await {
                match self_.process_event(event).await {
                    Ok(()) => (),
                    Err(err) => {
                        error!(
                            "hydra-controller: error: {}; will restart in {:?}…",
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

    async fn send_delayed(&self, event: Event, delay: Duration) {
        let event_tx = self.event_tx.clone();
        tokio::spawn(async move {
            tokio::time::sleep(delay).await;
            event_tx.send(event).await
        });
    }

    async fn process_event(&mut self, event: Event) -> Result<()> {
        match event {
            Event::Restart => {
                info!("hydra-controller: starting…");

                let potential_fuel = self
                    .lovelace_on_payment_skey(&self.config.cardano_signing_key)
                    .await?;
                if potential_fuel < Self::MIN_FUEL_LOVELACE {
                    Err(anyhow!(
                        "hydra-controller: {} ADA is too little for the Hydra L1 fees on the enterprise address associated with {:?}. Please provide at least {} ADA",
                        potential_fuel as f64 / 1_000_000.0,
                        self.config.cardano_signing_key,
                        Self::MIN_FUEL_LOVELACE as f64 / 1_000_000.0,
                    ))?
                }

                info!(
                    "hydra-controller: fuel on cardano_signing_key: {:?} lovelace",
                    potential_fuel
                );

                self.gen_hydra_keys().await?;

                self.kex_requests
                    .send(KeyExchangeRequest {
                        machine_id: verifications::hashed_machine_id(),
                        platform_cardano_vkey: self.platform_cardano_vkey.clone(),
                        platform_hydra_vkey: verifications::read_json_file(
                            &self.config_dir.join("hydra.vk"),
                        )?,
                        accepted_platform_h2h_port: None,
                    })
                    .await?;

                // FIXME: resend the request periodically in case it gets lost – i.e. new `Event::KExTimeout`
            },

            Event::Terminate => {
                if let Some(pid) = self.hydra_pid {
                    verifications::sigterm(pid)?
                }
            },

            Event::KeyExchangeResponse(
                kex_resp @ KeyExchangeResponse {
                    kex_done: false, ..
                },
            ) => {
                if !(matches!(
                    verifications::is_tcp_port_free(kex_resp.gateway_h2h_port).await,
                    Ok(true)
                ) && matches!(
                    verifications::is_tcp_port_free(kex_resp.proposed_platform_h2h_port).await,
                    Ok(true)
                )) {
                    warn!(
                        "hydra-controller: the ports proposed by the Gateway are not free locally, will ask again"
                    );
                    self.send(Event::Restart).await
                } else {
                    self.kex_requests
                        .send(KeyExchangeRequest {
                            machine_id: verifications::hashed_machine_id(),
                            platform_cardano_vkey: self.platform_cardano_vkey.clone(),
                            platform_hydra_vkey: verifications::read_json_file(
                                &self.config_dir.join("hydra.vk"),
                            )?,
                            accepted_platform_h2h_port: Some(kex_resp.proposed_platform_h2h_port),
                        })
                        .await?;
                }
            },

            Event::KeyExchangeResponse(kex_resp @ KeyExchangeResponse { kex_done: true, .. }) => {
                self.start_hydra_node(kex_resp).await?;
                self.send_delayed(Event::TryToCommit, Duration::from_secs(3))
                    .await
            },

            Event::TryToCommit => {
                let status = verifications::fetch_head_tag(self.api_port).await;

                info!(
                    "hydra-controller: waiting for the Initial head status: status={:?}",
                    status
                );

                match status.as_deref() {
                    Err(_) => {
                        self.send_delayed(Event::TryToCommit, Duration::from_secs(3))
                            .await
                    },
                    Ok(status) => {
                        self.last_hydra_head_state = status.to_string();
                        if status == "Initial" {
                            info!(
                                "hydra-controller: submitting an empty Commit transaction to join the Hydra Head"
                            );
                            self.empty_commit_to_hydra(
                                self.api_port,
                                &self.config.cardano_signing_key,
                            )
                            .await?;
                        }
                        self.send_delayed(Event::MonitorStates, Duration::from_secs(5))
                            .await
                    },
                }
            },

            Event::MonitorStates => {
                let new_status = verifications::fetch_head_tag(self.api_port).await?;

                if new_status != self.last_hydra_head_state {
                    let old = self.last_hydra_head_state.clone();
                    let new = new_status.clone();
                    self.last_hydra_head_state = new_status;

                    info!("hydra-controller: state changed from {old} to {new}");

                    if new == "Initial" {
                        self.send_delayed(Event::TryToCommit, Duration::from_secs(1))
                            .await;
                        return Ok(());
                    }
                }

                self.send_delayed(Event::MonitorStates, Duration::from_secs(5))
                    .await
            },
        }
        Ok(())
    }

    async fn start_hydra_node(&mut self, kex_response: KeyExchangeResponse) -> Result<()> {
        use std::process::Stdio;
        use tokio::io::{AsyncBufReadExt, BufReader};

        self.api_port = verifications::find_free_tcp_port().await?;
        let metrics_port = verifications::find_free_tcp_port().await?;

        // FIXME: somehow do shutdown once we’re killed
        // cf. <https://github.com/IntersectMBO/cardano-node/blob/10.6.1/cardano-node/src/Cardano/Node/Handlers/Shutdown.hs#L123-L148>
        // cf. <https://input-output-rnd.slack.com/archives/C06J9HK7QCQ/p1764782397820079>
        // TODO: Write a ticket in `hydra-node`.

        let protocol_parameters_path = self.config_dir.join("protocol-parameters.json");
        verifications::write_json_if_changed(
            &protocol_parameters_path,
            &kex_response.protocol_parameters,
        )?;

        let gateway_hydra_vkey_path = self.config_dir.join("gateway-hydra.vk");
        verifications::write_json_if_changed(
            &gateway_hydra_vkey_path,
            &kex_response.gateway_hydra_vkey,
        )?;

        let gateway_cardano_vkey_path = self.config_dir.join("gateway-payment.vk");
        verifications::write_json_if_changed(
            &gateway_cardano_vkey_path,
            &kex_response.gateway_cardano_vkey,
        )?;

        let mut child = tokio::process::Command::new(&self.hydra_node_exe)
            .arg("--node-id")
            .arg("platform-node")
            .arg("--persistence-dir")
            .arg(self.config_dir.join("persistence"))
            .arg("--cardano-signing-key")
            .arg(&self.config.cardano_signing_key) // FIXME: copy it somewhere else in case the source file changes
            .arg("--hydra-signing-key")
            .arg(self.config_dir.join("hydra.sk"))
            .arg("--hydra-scripts-tx-id")
            .arg(&kex_response.hydra_scripts_tx_id)
            .arg("--ledger-protocol-parameters")
            .arg(&protocol_parameters_path)
            .arg("--contestation-period")
            .arg(format!("{}s", kex_response.contestation_period.as_secs()))
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
            .arg(format!("{}", self.api_port))
            .arg("--api-host")
            .arg("127.0.0.1")
            .arg("--listen")
            .arg(format!("127.0.0.1:{}", kex_response.proposed_platform_h2h_port))
            .arg("--peer")
            .arg(format!("127.0.0.1:{}", kex_response.gateway_h2h_port))
            .arg("--monitoring-port")
            .arg(format!("{metrics_port}"))
            .arg("--hydra-verification-key")
            .arg(gateway_hydra_vkey_path)
            .arg("--cardano-verification-key")
            .arg(gateway_cardano_vkey_path)
            .stdin(Stdio::null()) // FIXME: try an empty pipe, and see if it exitst on our `kill -9`
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        self.hydra_pid = child.id();

        let stdout = child.stdout.take().expect("child stdout");
        let stderr = child.stderr.take().expect("child stderr");

        tokio::spawn(async move {
            let mut lines = BufReader::new(stdout).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                debug!("hydra-node: {}", line);
            }
            debug!("hydra-node: stdout closed");
        });

        tokio::spawn(async move {
            let mut lines = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                info!("hydra-node: {}", line);
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
}
