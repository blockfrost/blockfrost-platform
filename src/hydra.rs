use crate::config::HydraConfig as HydraTomlConfig;
use crate::types::{AssetName, Network};
use anyhow::{Result, anyhow};
use serde::Deserialize;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

pub mod tunnel;
pub mod verifications;

// FIXME: this should most probably be back to the default of 600 seconds:
const CONTESTATION_PERIOD_SECONDS: Duration = Duration::from_secs(60);

// FIXME: shouldn’t this be multiplied by `max_concurrent_hydra_nodes`?
const MIN_FUEL_LOVELACE: u64 = 15_000_000;

/// After cloning, it still represents the same set of [`HydraController`]s.
#[derive(Clone, Debug)]
pub struct HydrasManager {
    config: HydraConfig,
    /// This is `Arc<Arc<()>>` because we want all clones of the controller to only hold a single copy.
    #[allow(clippy::redundant_allocation)]
    controller_counter: Arc<Arc<()>>,
}

impl HydrasManager {
    pub async fn new(config: &HydraTomlConfig, network: &Network) -> Result<Self> {
        // Let’s add some ε of 1% just to be sure about rounding etc.
        let minimal_commit: f64 = 1.01
            * (config.lovelace_per_request
                * config.requests_per_microtransaction
                * config.microtransactions_per_fanout) as f64
            / 1_000_000.0;
        if config.commit_ada < minimal_commit {
            Err(anyhow!(
                "hydras-manager: Please make sure that configured commit_ada ≥ lovelace_per_request * requests_per_microtransaction * microtransactions_per_fanout."
            ))?
        }

        Ok(Self {
            config: HydraConfig::load(config.clone(), network).await?,
            controller_counter: Arc::new(Arc::new(())),
        })
    }

    pub async fn initialize_key_exchange(
        &self,
        originator: &AssetName,
        req: KeyExchangeRequest,
    ) -> Result<KeyExchangeResponse> {
        if req.accepted_platform_h2h_port.is_some() {
            Err(anyhow!(
                "`accepted_platform_h2h_port` must not be set in `initialize_key_exchange`"
            ))?
        }

        let cur_count = Arc::strong_count(&self.controller_counter.as_ref()).saturating_sub(1); // subtract the manager
        if cur_count as u64 >= self.config.toml.max_concurrent_hydra_nodes {
            let err = anyhow!(
                "Too many concurrent `hydra-node`s already running. You can increase the limit in config."
            );
            warn!("{}", err);
            Err(err)?
        }

        let have_funds: f64 = self
            .config
            .lovelace_on_addr(&self.config.gateway_cardano_addr)
            .await? as f64
            / 1_000_000.0;
        let required_funds_ada: f64 =
            self.config.toml.commit_ada + (MIN_FUEL_LOVELACE as f64 / 1_000_000.0);
        if have_funds < required_funds_ada {
            let err = anyhow!(
                "hydra-controller: {:?}: {} ADA is too little for the Hydra L1 fees and committed funds on the enterprise address associated with {:?}. Please provide at least {} ADA",
                originator,
                have_funds,
                self.config.toml.cardano_signing_key,
                required_funds_ada,
            );
            error!("{}", err);
            Err(err)?
        }
        info!(
            "hydra-controller: {:?}: funds on cardano_signing_key: {:?} ADA",
            originator, have_funds
        );

        use verifications::{find_free_tcp_port, read_json_file};

        let config_dir = mk_config_dir(&self.config.network, &originator)?;
        self.config.gen_hydra_keys(&config_dir).await?;

        Ok(KeyExchangeResponse {
            gateway_cardano_vkey: self.config.gateway_cardano_vkey.clone(),
            gateway_hydra_vkey: read_json_file(&config_dir.join("hydra.vk"))?,
            hydra_scripts_tx_id: hydra_scripts_tx_id(&self.config.network).to_string(),
            protocol_parameters: self.config.protocol_parameters.clone(),
            contestation_period: CONTESTATION_PERIOD_SECONDS,
            proposed_platform_h2h_port: find_free_tcp_port().await?,
            gateway_h2h_port: find_free_tcp_port().await?,
            kex_done: false,
        })
    }

    /// You should first call [`Self::initialize_key_exchange`], and then this
    /// function with the initial request/response pair.
    pub async fn spawn_new(
        &self,
        originator: &AssetName,
        reward_addr: &str,
        initial: (KeyExchangeRequest, KeyExchangeResponse),
        final_req: KeyExchangeRequest,
    ) -> Result<(HydraController, KeyExchangeResponse)> {
        if initial.0
            != (KeyExchangeRequest {
                accepted_platform_h2h_port: None,
                ..final_req.clone()
            })
        {
            Err(anyhow!(
                "The 2nd `KeyExchangeRequest` must be the same as the 1st one."
            ))?
        }

        if final_req.accepted_platform_h2h_port != Some(initial.1.proposed_platform_h2h_port) {
            Err(anyhow!(
                "The Platform must accept the same port that was proposed to it."
            ))?
        }

        // Clone first, to prevent the nastier race condition:
        let maybe_new = Arc::clone(self.controller_counter.as_ref());
        let new_count = Arc::strong_count(&self.controller_counter.as_ref()).saturating_sub(1); // subtract the manager
        if new_count as u64 > self.config.toml.max_concurrent_hydra_nodes {
            Err(anyhow!(
                "Too many concurrent `hydra-node`s already running. You can increase the limit in config."
            ))?
        }

        if !(matches!(
            verifications::is_tcp_port_free(initial.1.gateway_h2h_port).await,
            Ok(true)
        ) && matches!(
            verifications::is_tcp_port_free(initial.1.proposed_platform_h2h_port).await,
            Ok(true)
        )) {
            Err(anyhow!(
                "The exchanged ports are no longer free on the gateway, please perform another KEx."
            ))?
        }

        let final_resp = KeyExchangeResponse {
            kex_done: true,
            ..initial.1
        };

        let ctl = HydraController::spawn(
            self.config.clone(),
            originator.clone(),
            reward_addr.to_string(),
            maybe_new,
            final_req,
            final_resp.clone(),
        )
        .await?;

        Ok((ctl, final_resp))
    }
}

#[derive(Debug, Deserialize, Clone)]
struct HydraConfig {
    pub toml: HydraTomlConfig,
    pub network: Network,
    pub hydra_node_exe: String,
    pub cardano_cli_exe: String,
    pub gateway_cardano_vkey: serde_json::Value,
    pub gateway_cardano_addr: String,
    pub protocol_parameters: serde_json::Value,
}

impl HydraConfig {
    pub async fn load(toml: HydraTomlConfig, network: &Network) -> Result<Self> {
        let hydra_node_exe =
            crate::find_libexec::find_libexec("hydra-node", "HYDRA_NODE_PATH", &["--version"])
                .map_err(|e| anyhow!(e))?;
        let cardano_cli_exe =
            crate::find_libexec::find_libexec("cardano-cli", "CARDANO_CLI_PATH", &["version"])
                .map_err(|e| anyhow!(e))?;
        let self_ = Self {
            toml,
            network: network.clone(),
            hydra_node_exe,
            cardano_cli_exe,
            gateway_cardano_vkey: serde_json::Value::Null,
            gateway_cardano_addr: String::new(),
            protocol_parameters: serde_json::Value::Null,
        };
        let gateway_cardano_addr = self_
            .derive_enterprise_address_from_skey(&self_.toml.cardano_signing_key)
            .await?;
        let gateway_cardano_vkey = self_
            .derive_vkey_from_skey(&self_.toml.cardano_signing_key)
            .await?;
        let protocol_parameters = self_.gen_protocol_parameters().await?;
        let self_ = Self {
            gateway_cardano_vkey,
            gateway_cardano_addr,
            protocol_parameters,
            ..self_
        };
        Ok(self_)
    }
}

/// Runs a `hydra-node` and sets up an L2 network with the Platform for microtransactions.
///
/// You can safely clone it, and the clone will represent the same `hydra-node` etc.
#[derive(Clone)]
pub struct HydraController {
    event_tx: mpsc::Sender<Event>,
    originator: AssetName,
    _controller_counter: Arc<()>,
}

// FIXME: send a Quit event on `drop()` of all controller instances

#[derive(serde::Deserialize, serde::Serialize, Debug, PartialEq, Eq, Clone)]
pub struct KeyExchangeRequest {
    pub platform_cardano_vkey: serde_json::Value,
    pub platform_hydra_vkey: serde_json::Value,
    pub accepted_platform_h2h_port: Option<u16>,
}

#[derive(serde::Deserialize, serde::Serialize, Debug, PartialEq, Eq, Clone)]
pub struct KeyExchangeResponse {
    pub gateway_cardano_vkey: serde_json::Value,
    pub gateway_hydra_vkey: serde_json::Value,
    pub hydra_scripts_tx_id: String,
    pub protocol_parameters: serde_json::Value,
    pub contestation_period: Duration,
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

impl HydraController {
    async fn spawn(
        config: HydraConfig,
        originator: AssetName,
        reward_addr: String,
        controller_counter: Arc<()>,
        kex_req: KeyExchangeRequest,
        kex_resp: KeyExchangeResponse,
    ) -> Result<Self> {
        let event_tx =
            State::spawn(config, originator.clone(), reward_addr, kex_req, kex_resp).await?;
        Ok(Self {
            event_tx,
            originator,
            _controller_counter: controller_counter,
        })
    }

    // FIXME: this is too primitive
    pub fn is_alive(&self) -> bool {
        !self.event_tx.is_closed()
    }

    pub async fn account_one_request(&self) {
        self.event_tx
            .send(Event::AccountOneRequest)
            .await
            .unwrap_or_else(|_| {
                error!(
                    "hydra-controller: {:?}: failed to account one request: event channel closed",
                    self.originator
                )
            })
    }
}

enum Event {
    Restart,
    TryToOpenHead,
    TryToCommit,
    WaitForOpen,
    AccountOneRequest,
    WaitForUtxoCount,
    TryToClose,
    WaitForClosed { retries_before_reclose: u64 },
    DoFanout,
    WaitForIdleAfterClose,
}

fn mk_config_dir(network: &Network, originator: &AssetName) -> Result<PathBuf> {
    let config_dir = dirs::config_dir()
        .ok_or(anyhow!("`dirs::config_dir()` returned `None`"))?
        .join("blockfrost-gateway")
        .join("hydra")
        .join(network.as_str())
        .join(originator.as_str());
    std::fs::create_dir_all(&config_dir)?;
    Ok(config_dir)
}

// FIXME: don’t construct all key and other paths manually, keep them in a single place
struct State {
    config: HydraConfig,
    originator: AssetName,
    reward_addr: String,
    config_dir: PathBuf,
    event_tx: mpsc::Sender<Event>,
    kex_req: KeyExchangeRequest,
    kex_resp: KeyExchangeResponse,
    api_port: u16,
    metrics_port: u16,
    hydra_peers_connected: bool, // FIXME: they can become disconnected…
    hydra_head_open: bool,
    accounted_requests: u64,
    sent_microtransactions: u64,
    commit_wallet_skey: PathBuf,
    commit_wallet_addr: String,
    is_closing: bool,
}

impl State {
    const RESTART_DELAY: Duration = Duration::from_secs(5);

    async fn spawn(
        config: HydraConfig,
        originator: AssetName,
        reward_addr: String,
        kex_req: KeyExchangeRequest,
        kex_resp: KeyExchangeResponse,
    ) -> Result<mpsc::Sender<Event>> {
        let config_dir = mk_config_dir(&config.network, &originator)?;

        let (event_tx, mut event_rx) = mpsc::channel::<Event>(32);

        let mut self_ = Self {
            config,
            originator,
            reward_addr,
            config_dir,
            event_tx: event_tx.clone(),
            kex_req,
            kex_resp,
            api_port: 0,
            metrics_port: 0,
            hydra_peers_connected: false,
            hydra_head_open: false,
            accounted_requests: 0,
            sent_microtransactions: 0,
            commit_wallet_skey: PathBuf::new(),
            commit_wallet_addr: String::new(),
            is_closing: true,
        };

        self_.send(Event::Restart).await;

        tokio::spawn(async move {
            while let Some(event) = event_rx.recv().await {
                match self_.process_event(event).await {
                    Ok(()) => (),
                    Err(err) => {
                        error!(
                            "hydra-controller: {:?}: error: {}; will restart in {:?}…",
                            self_.originator,
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
                info!("hydra-controller: {:?}: starting…", self.originator);
                self.start_hydra_node().await?;
                self.send_delayed(Event::TryToOpenHead, Duration::from_secs(1))
                    .await
            },

            Event::TryToOpenHead => {
                let ready = verifications::prometheus_metric_at_least(
                    &format!("http://127.0.0.1:{}/metrics", self.metrics_port),
                    "hydra_head_peers_connected",
                    1.0,
                )
                .await?;

                info!(
                    "hydra-controller: {:?}: waiting for hydras to connect: ready={:?}",
                    self.originator, ready
                );

                if ready {
                    self.hydra_peers_connected = true;

                    verifications::send_one_websocket_msg(
                        &format!("http://127.0.0.1:{}/", self.api_port),
                        serde_json::json!({"tag":"Init"}),
                        Duration::from_secs(2),
                    )
                    .await?;

                    self.send_delayed(Event::TryToCommit, Duration::from_secs(3))
                        .await
                } else {
                    self.send_delayed(Event::TryToOpenHead, Duration::from_secs(1))
                        .await
                }
            },

            Event::TryToCommit => {
                let status = verifications::fetch_head_tag(self.api_port).await?;

                info!(
                    "hydra-controller: {:?}: waiting for the Initial head status: status={:?}",
                    self.originator, status
                );

                if status == "Initial" {
                    let commit_wallet = self.config_dir.join("commit-funds");
                    self.commit_wallet_skey = commit_wallet.with_extension("sk");

                    if std::fs::exists(&self.commit_wallet_skey)? {
                        warn!(
                            "hydra-controller: {:?}: commit wallet {:?} already exists, skipping the Commit transaction",
                            self.originator, self.commit_wallet_skey
                        );
                    } else {
                        self.config.new_cardano_keypair(&commit_wallet).await?;
                        let commit_wallet_addr = self
                            .config
                            .derive_enterprise_address_from_skey(
                                &self.config_dir.join("commit-funds.sk"),
                            )
                            .await?;
                        self.config
                            .fund_address(
                                &self.config.gateway_cardano_addr,
                                &commit_wallet_addr,
                                (self.config.toml.commit_ada * 1_000_000.0).round() as u64,
                                &self.config.toml.cardano_signing_key,
                            )
                            .await?;

                        self.config
                            .commit_all_utxo_to_hydra(
                                &commit_wallet_addr,
                                self.api_port,
                                &self.commit_wallet_skey,
                            )
                            .await?;

                        self.send_delayed(Event::WaitForOpen, Duration::from_secs(3))
                            .await
                    }
                } else {
                    self.send_delayed(Event::TryToCommit, Duration::from_secs(3))
                        .await
                }
            },

            Event::WaitForOpen => {
                let status = verifications::fetch_head_tag(self.api_port).await?;
                info!(
                    "hydra-controller: {:?}: waiting for the Open head status: status={:?}",
                    self.originator, status
                );
                if status == "Open" {
                    self.hydra_head_open = true;
                } else {
                    self.send_delayed(Event::WaitForOpen, Duration::from_secs(3))
                        .await
                }
            },

            Event::AccountOneRequest => {
                self.accounted_requests += 1;

                if self.accounted_requests >= self.config.toml.requests_per_microtransaction {
                    if self.is_closing {
                        warn!(
                            "hydra-controller: {:?}: would send a microtransaction, but the Hydra Head state is currently closing for `Fanout` (backlog of requests: {})",
                            self.originator, self.accounted_requests
                        )
                    } else if self.hydra_head_open {
                        info!(
                            "hydra-controller: {:?}: sending a microtransaction",
                            self.originator
                        );
                        let amount_lovelace: u64 =
                            self.accounted_requests * self.config.toml.lovelace_per_request;
                        self.config
                            .send_hydra_transaction(
                                self.api_port,
                                &self.commit_wallet_addr,
                                &self.reward_addr,
                                &self.commit_wallet_skey,
                                amount_lovelace,
                            )
                            .await?;

                        self.accounted_requests = 0;
                        self.sent_microtransactions += 1;

                        if self.sent_microtransactions
                            >= self.config.toml.microtransactions_per_fanout
                        {
                            self.is_closing = true;
                            self.send_delayed(Event::WaitForUtxoCount, Duration::from_secs(3))
                                .await;
                        }
                    } else {
                        warn!(
                            "hydra-controller: {:?}: would send a microtransaction, but the Hydra Head state is still not `Open` (backlog of requests: {})",
                            self.originator, self.accounted_requests
                        )
                    }
                }
            },

            Event::WaitForUtxoCount => {
                // XXX: `1 +`, because we also have the source UTxO of the `commit_wallet`
                let expected_count = 1 + self.sent_microtransactions;
                let current_count = self.config.hydra_utxo_count(self.api_port).await?;

                if current_count >= expected_count {
                    info!(
                        "hydra-controller: {:?}: got correct L2 UTxO count, will Close now…",
                        self.originator
                    );
                    self.send_delayed(Event::TryToClose, Duration::from_secs(1))
                        .await;
                } else {
                    warn!(
                        "hydra-controller: {:?}: still have incorrect L2 UTxO count: {}, expected {}",
                        self.originator, current_count, expected_count
                    );
                    self.send_delayed(Event::WaitForUtxoCount, Duration::from_secs(3))
                        .await;
                }
            },

            Event::TryToClose => {
                verifications::send_one_websocket_msg(
                    &format!("http://127.0.0.1:{}", self.api_port),
                    serde_json::json!({"tag":"Close"}),
                    Duration::from_secs(2),
                )
                .await?;
                self.send_delayed(
                    Event::WaitForClosed {
                        retries_before_reclose: 10,
                    },
                    Duration::from_secs(3),
                )
                .await;
            },

            Event::WaitForClosed {
                retries_before_reclose,
            } => {
                let status = verifications::fetch_head_tag(self.api_port).await?;
                info!(
                    "hydra-controller: {:?}: waiting for the Closed head status: status={:?}",
                    self.originator, status
                );
                if status == "Closed" {
                    self.send_delayed(Event::DoFanout, Duration::from_secs(1))
                        .await
                } else {
                    if retries_before_reclose <= 1 {
                        let invalidity_period = (2 + 1) * CONTESTATION_PERIOD_SECONDS;
                        info!(
                            "hydra-controller: {:?}: will wait through the invalidity period ({:?}) before requesting `Fanout`",
                            self.originator, invalidity_period,
                        );
                        self.send_delayed(Event::TryToClose, invalidity_period)
                            .await;
                    } else {
                        self.send_delayed(
                            Event::WaitForClosed {
                                retries_before_reclose: retries_before_reclose - 1,
                            },
                            Duration::from_secs(3),
                        )
                        .await
                    }
                }
            },

            Event::DoFanout => {
                info!(
                    "hydra-controller: {:?}: requesting `Fanout`",
                    self.originator,
                );
                verifications::send_one_websocket_msg(
                    &format!("http://127.0.0.1:{}", self.api_port),
                    serde_json::json!({"tag":"Fanout"}),
                    Duration::from_secs(2),
                )
                .await?;
                self.send_delayed(Event::WaitForIdleAfterClose, Duration::from_secs(3))
                    .await;
            },

            Event::WaitForIdleAfterClose => {
                let status = verifications::fetch_head_tag(self.api_port).await?;
                info!(
                    "hydra-controller: {:?}: waiting for the Idle head status (after Fanout): status={:?}",
                    self.originator, status
                );
                if status == "Idle" {
                    info!(
                        "hydra-controller: {:?}: re-initializing the Hydra Head for another L2 session",
                        self.originator,
                    );

                    self.send_delayed(Event::TryToOpenHead, Duration::from_secs(3))
                        .await;
                } else {
                    self.send_delayed(Event::WaitForIdleAfterClose, Duration::from_secs(3))
                        .await;
                }
            },
        }
        Ok(())
    }

    async fn start_hydra_node(&mut self) -> Result<()> {
        use std::process::Stdio;
        use tokio::io::{AsyncBufReadExt, BufReader};

        self.api_port = verifications::find_free_tcp_port().await?;
        self.metrics_port = verifications::find_free_tcp_port().await?;

        // FIXME: somehow do shutdown once we’re killed
        // cf. <https://github.com/IntersectMBO/cardano-node/blob/10.6.1/cardano-node/src/Cardano/Node/Handlers/Shutdown.hs#L123-L148>
        // cf. <https://input-output-rnd.slack.com/archives/C06J9HK7QCQ/p1764782397820079>
        // TODO: Write a ticket in `hydra-node`.

        let protocol_parameters_path = self.config_dir.join("protocol-parameters.json");
        verifications::write_json_if_changed(
            &protocol_parameters_path,
            &self.kex_resp.protocol_parameters,
        )?;

        let platform_hydra_vkey_path = self.config_dir.join("platform-hydra.vk");
        verifications::write_json_if_changed(
            &platform_hydra_vkey_path,
            &self.kex_req.platform_hydra_vkey,
        )?;

        let platform_cardano_vkey_path = self.config_dir.join("platform-payment.vk");
        verifications::write_json_if_changed(
            &platform_cardano_vkey_path,
            &self.kex_req.platform_cardano_vkey,
        )?;

        let mut child = tokio::process::Command::new(&self.config.hydra_node_exe)
            .arg("--node-id")
            .arg("platform-node")
            .arg("--persistence-dir")
            .arg(self.config_dir.join("persistence"))
            .arg("--cardano-signing-key")
            .arg(&self.config.toml.cardano_signing_key) // FIXME: copy it somewhere else in case the source file changes
            .arg("--hydra-signing-key")
            .arg(self.config_dir.join("hydra.sk"))
            .arg("--hydra-scripts-tx-id")
            .arg(&self.kex_resp.hydra_scripts_tx_id)
            .arg("--ledger-protocol-parameters")
            .arg(&protocol_parameters_path) // FIXME: copy it somewhere else in case the source file changes
            .arg("--contestation-period")
            .arg(format!("{}s", self.kex_resp.contestation_period.as_secs()))
            .args(if self.config.network == Network::Mainnet {
                vec!["-mainnet".to_string()]
            } else {
                vec![
                    "--testnet-magic".to_string(),
                    format!("{}", self.config.network.network_magic()),
                ]
            })
            .arg("--node-socket")
            .arg(&self.config.toml.node_socket_path)
            .arg("--api-port")
            .arg(format!("{}", self.api_port))
            .arg("--api-host")
            .arg("127.0.0.1")
            .arg("--listen")
            .arg(format!("127.0.0.1:{}", self.kex_resp.gateway_h2h_port))
            .arg("--peer")
            .arg(format!("127.0.0.1:{}", self.kex_resp.proposed_platform_h2h_port))
            .arg("--monitoring-port")
            .arg(format!("{}", self.metrics_port))
            .arg("--hydra-verification-key")
            .arg(platform_hydra_vkey_path)
            .arg("--cardano-verification-key")
            .arg(platform_cardano_vkey_path)
            .stdin(Stdio::null()) // FIXME: try an empty pipe, and see if it exitst on our `kill -9`
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

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

pub fn hydra_scripts_tx_id(network: &Network) -> &'static str {
    // FIXME: also define them in a `build.rs` script without Nix – consult
    // `flake.lock` to get the exact Hydra version.
    use Network::*;
    match network {
        Mainnet => env!("HYDRA_SCRIPTS_TX_ID_MAINNET"),
        Preprod => env!("HYDRA_SCRIPTS_TX_ID_PREPROD"),
        Preview => env!("HYDRA_SCRIPTS_TX_ID_PREVIEW"),
    }
}
