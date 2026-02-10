use anyhow::{Result, anyhow};
use std::time::Duration;
use std::{path::PathBuf, sync::Arc};
use tokio::sync::{Mutex, mpsc, oneshot, watch};
use tracing::{debug, error, info, warn};

pub mod verifications;

/// Runs a `hydra-node` and sets up an L2 network with the Gateway for microtransactions.
///
/// You can safely clone it, and the clone will represent the same `hydra-node` etc.
#[derive(Clone)]
pub struct HydraController {
    event_tx: mpsc::Sender<Event>,
    api_port_rx: watch::Receiver<Option<u16>>,
}

#[derive(Clone, Debug)]
pub struct HydraClientConfig {
    pub cardano_signing_key: PathBuf,
    pub commit_ada: f64,
    pub lovelace_per_request: u64,
    pub requests_per_microtransaction: u64,
    pub microtransactions_per_fanout: u64,
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
    #[serde(default)]
    pub lovelace_per_request: u64,
    #[serde(default)]
    pub requests_per_microtransaction: u64,
    #[serde(default)]
    pub microtransactions_per_fanout: u64,
}

pub struct TerminateRequest;

impl HydraController {
    // FIXME: refactor
    #[allow(clippy::too_many_arguments)]
    pub async fn spawn(
        config: HydraClientConfig,
        network: crate::types::Network,
        node_socket_path: String,
        reward_address: String,
        health_errors: Arc<Mutex<Vec<String>>>,
        kex_requests: mpsc::Sender<KeyExchangeRequest>,
        kex_responses: mpsc::Receiver<KeyExchangeResponse>,
        terminate_reqs: mpsc::Receiver<TerminateRequest>,
    ) -> Result<Self> {
        let (event_tx, api_port_rx) = State::spawn(
            config,
            network,
            node_socket_path,
            reward_address,
            health_errors,
            kex_requests,
            kex_responses,
            terminate_reqs,
        )
        .await?;
        Ok(Self {
            event_tx,
            api_port_rx,
        })
    }

    pub async fn terminate(&self) {
        let _ = self.event_tx.send(Event::Terminate).await;
    }

    pub async fn account_one_request(&self) {
        let _ = self.event_tx.send(Event::AccountOneRequest).await;
    }

    pub async fn send_payment(&self, amount_lovelace: u64, receiver_addr: String) -> Result<()> {
        let (tx, rx) = oneshot::channel();
        let _ = self
            .event_tx
            .send(Event::SendPayment {
                amount_lovelace,
                receiver_addr,
                respond_to: tx,
            })
            .await;
        rx.await
            .unwrap_or_else(|_| Err(anyhow!("payment request cancelled")))
    }

    pub fn api_port(&self) -> Option<u16> {
        *self.api_port_rx.borrow()
    }

    pub async fn wait_api_port(&self) -> u16 {
        let mut rx = self.api_port_rx.clone();
        loop {
            if let Some(port) = *rx.borrow() {
                return port;
            }
            if rx.changed().await.is_err() {
                return 0;
            }
        }
    }
}

enum Event {
    Restart,
    Terminate,
    KeyExchangeResponse(KeyExchangeResponse),
    FundCommitAddr,
    TryToCommit,
    WaitForOpen,
    MonitorStates,
    AccountOneRequest,
    SendPayment {
        amount_lovelace: u64,
        receiver_addr: String,
        respond_to: oneshot::Sender<Result<()>>,
    },
}

#[derive(Debug, Clone, Copy)]
struct PaymentParams {
    lovelace_per_request: u64,
    requests_per_microtransaction: u64,
    microtransactions_per_fanout: u64,
}

impl PaymentParams {
    fn from_config(config: &HydraClientConfig) -> Self {
        Self {
            lovelace_per_request: config.lovelace_per_request,
            requests_per_microtransaction: config.requests_per_microtransaction,
            microtransactions_per_fanout: config.microtransactions_per_fanout,
        }
    }

    fn update_from_response(&mut self, resp: &KeyExchangeResponse) {
        if resp.lovelace_per_request > 0 {
            self.lovelace_per_request = resp.lovelace_per_request;
        }
        if resp.requests_per_microtransaction > 0 {
            self.requests_per_microtransaction = resp.requests_per_microtransaction;
        }
        if resp.microtransactions_per_fanout > 0 {
            self.microtransactions_per_fanout = resp.microtransactions_per_fanout;
        }
    }
}

// FIXME: don’t construct all key and other paths manually, keep them in a single place
struct State {
    config: HydraClientConfig,
    network: crate::types::Network,
    node_socket_path: String,
    platform_cardano_vkey: serde_json::Value,
    _reward_address: String,
    _health_errors: Arc<Mutex<Vec<String>>>,
    kex_requests: mpsc::Sender<KeyExchangeRequest>,
    api_port: u16,
    api_port_tx: watch::Sender<Option<u16>>,
    hydra_node_exe: String,
    cardano_cli_exe: String,
    config_dir: PathBuf,
    event_tx: mpsc::Sender<Event>,
    last_hydra_head_state: String,
    hydra_head_open: bool,
    accounted_requests: u64,
    commit_wallet_skey: PathBuf,
    commit_wallet_addr: String,
    hydra_pid: Option<u32>,
    payment_params: PaymentParams,
}

impl State {
    const RESTART_DELAY: std::time::Duration = std::time::Duration::from_secs(5);
    const MIN_FUEL_LOVELACE: u64 = 15_000_000;

    // FIXME: refactor
    #[allow(clippy::too_many_arguments)]
    async fn spawn(
        config: HydraClientConfig,
        network: crate::types::Network,
        node_socket_path: String,
        reward_address: String,
        health_errors: Arc<Mutex<Vec<String>>>,
        kex_requests: mpsc::Sender<KeyExchangeRequest>,
        kex_responses: mpsc::Receiver<KeyExchangeResponse>,
        terminate_reqs: mpsc::Receiver<TerminateRequest>,
    ) -> Result<(mpsc::Sender<Event>, watch::Receiver<Option<u16>>)> {
        let hydra_node_exe =
            crate::find_libexec::find_libexec("hydra-node", "HYDRA_NODE_PATH", &["--version"])
                .map_err(|e| anyhow!(e))?;
        let cardano_cli_exe =
            crate::find_libexec::find_libexec("cardano-cli", "CARDANO_CLI_PATH", &["version"])
                .map_err(|e| anyhow!(e))?;

        // FIXME: config dir prob. needs to be gateway specific? Test it!
        let gateway_prefix = "_default";

        let config_dir = dirs::config_dir()
            .expect("Could not determine config directory")
            .join("blockfrost-platform")
            .join("hydra")
            .join(network.as_str())
            .join(gateway_prefix);

        let (event_tx, mut event_rx) = mpsc::channel::<Event>(32);
        let (api_port_tx, api_port_rx) = watch::channel(None);

        let payment_params = PaymentParams::from_config(&config);
        let self_ = Self {
            config,
            network,
            node_socket_path,
            platform_cardano_vkey: serde_json::Value::Null,
            _reward_address: reward_address,
            _health_errors: health_errors,
            kex_requests,
            api_port: 0,
            api_port_tx,
            hydra_node_exe,
            cardano_cli_exe,
            config_dir,
            event_tx: event_tx.clone(),
            last_hydra_head_state: String::new(),
            hydra_head_open: false,
            accounted_requests: 0,
            commit_wallet_skey: PathBuf::new(),
            commit_wallet_addr: String::new(),
            hydra_pid: None,
            payment_params,
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

        Ok((event_tx, api_port_rx))
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
                let required_funds =
                    Self::MIN_FUEL_LOVELACE + (self.config.commit_ada * 1_000_000.0) as u64;
                if potential_fuel < required_funds {
                    Err(anyhow!(
                        "hydra-controller: {} ADA is too little for the Hydra L1 fees and committed funds on the enterprise address associated with {:?}. Please provide at least {} ADA",
                        potential_fuel as f64 / 1_000_000.0,
                        self.config.cardano_signing_key,
                        required_funds as f64 / 1_000_000.0,
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

                self.hydra_head_open = false;
                self.accounted_requests = 0;
                self.commit_wallet_addr.clear();
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
                self.payment_params.update_from_response(&kex_resp);
                self.start_hydra_node(kex_resp).await?;
                self.send_delayed(Event::FundCommitAddr, Duration::from_secs(3))
                    .await
            },

            Event::FundCommitAddr => {
                let status = verifications::fetch_head_tag(self.api_port).await?;

                info!(
                    "hydra-controller: waiting for the Initial head status: status={:?}",
                    status
                );

                if status == "Initial" || status == "Open" {
                    let commit_wallet = self.config_dir.join("commit-funds");
                    self.commit_wallet_skey = commit_wallet.with_extension("sk");

                    if !std::fs::exists(&self.commit_wallet_skey)? {
                        self.new_cardano_keypair(&commit_wallet).await?;
                    }

                    self.commit_wallet_addr = self
                        .derive_enterprise_address_from_skey(&self.commit_wallet_skey)
                        .await?;

                    if status == "Initial" {
                        let payer_addr = self
                            .derive_enterprise_address_from_skey(&self.config.cardano_signing_key)
                            .await?;
                        self.fund_address(
                            &payer_addr,
                            &self.commit_wallet_addr,
                            (self.config.commit_ada * 1_000_000.0).round() as u64,
                            &self.config.cardano_signing_key,
                        )
                        .await?;
                        self.send_delayed(Event::TryToCommit, Duration::from_secs(3))
                            .await;
                    } else {
                        self.hydra_head_open = true;
                        self.send_delayed(Event::MonitorStates, Duration::from_secs(5))
                            .await;
                    }
                } else {
                    self.send_delayed(Event::FundCommitAddr, Duration::from_secs(3))
                        .await;
                }
            },

            Event::TryToCommit => {
                let commit_wallet_lovelace =
                    self.lovelace_on_addr(&self.commit_wallet_addr).await?;
                let lovelace_needed = 0.99 * self.config.commit_ada * 1_000_000.0;

                info!(
                    "hydra-controller: waiting for enough lovelace (> {}) to appear on the commit address: lovelace={:?}",
                    lovelace_needed.round(),
                    commit_wallet_lovelace
                );

                if commit_wallet_lovelace as f64 >= lovelace_needed {
                    info!(
                        "hydra-controller: submitting a Commit transaction to join the Hydra Head"
                    );
                    self.commit_all_utxo_to_hydra(
                        &self.commit_wallet_addr,
                        self.api_port,
                        &self.commit_wallet_skey,
                    )
                    .await?;
                    self.send_delayed(Event::WaitForOpen, Duration::from_secs(3))
                        .await;
                } else {
                    self.send_delayed(Event::TryToCommit, Duration::from_secs(3))
                        .await;
                }
            },

            Event::WaitForOpen => {
                let status = verifications::fetch_head_tag(self.api_port).await?;
                info!(
                    "hydra-controller: waiting for the Open head status: status={:?}",
                    status
                );
                if status == "Open" {
                    self.hydra_head_open = true;
                } else {
                    self.hydra_head_open = false;
                    self.send_delayed(Event::WaitForOpen, Duration::from_secs(3))
                        .await;
                }
                self.send_delayed(Event::MonitorStates, Duration::from_secs(5))
                    .await;
            },

            Event::MonitorStates => {
                let new_status = verifications::fetch_head_tag(self.api_port).await?;

                if new_status != self.last_hydra_head_state {
                    let old = self.last_hydra_head_state.clone();
                    let new = new_status.clone();
                    self.last_hydra_head_state = new_status;

                    info!("hydra-controller: state changed from {old} to {new}");
                }

                self.hydra_head_open = self.last_hydra_head_state == "Open";

                if self.last_hydra_head_state == "Initial" {
                    self.send_delayed(Event::FundCommitAddr, Duration::from_secs(1))
                        .await;
                    return Ok(());
                }

                self.send_delayed(Event::MonitorStates, Duration::from_secs(5))
                    .await
            },

            Event::AccountOneRequest => {
                self.accounted_requests = self.accounted_requests.saturating_add(1);
            },

            Event::SendPayment {
                amount_lovelace,
                receiver_addr,
                respond_to,
            } => {
                let res = if !self.hydra_head_open {
                    Err(anyhow!("hydra head is not open"))
                } else if self.commit_wallet_addr.is_empty() {
                    Err(anyhow!("commit wallet is not initialized"))
                } else {
                    self.send_hydra_transaction(
                        self.api_port,
                        &self.commit_wallet_addr,
                        &receiver_addr,
                        &self.commit_wallet_skey,
                        amount_lovelace,
                    )
                    .await
                };
                let _ = respond_to.send(res);
            },
        }
        Ok(())
    }

    async fn start_hydra_node(&mut self, kex_response: KeyExchangeResponse) -> Result<()> {
        use std::process::Stdio;
        use tokio::io::{AsyncBufReadExt, BufReader};

        self.api_port = verifications::find_free_tcp_port().await?;
        let metrics_port = verifications::find_free_tcp_port().await?;
        let _ = self.api_port_tx.send(Some(self.api_port));

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
            .args(if self.network == crate::types::Network::Mainnet {
                vec!["-mainnet".to_string()]
            } else {
                vec![
                    "--testnet-magic".to_string(),
                    format!("{}", self.network.network_magic()),
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
