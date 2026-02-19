use crate::config::HydraConfig as HydraTomlConfig;
use crate::types::Network;
use anyhow::{Result, anyhow};
use serde::Deserialize;
use std::path::PathBuf;
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering},
};
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

pub mod tunnel2;
pub mod verifications;

// FIXME: this should most probably be back to the default of 600 seconds:
const CONTESTATION_PERIOD_SECONDS: Duration = Duration::from_secs(60);

// FIXME: shouldn’t this be multiplied by `max_concurrent_hydra_nodes`?
const MIN_FUEL_LOVELACE: u64 = 15_000_000;

// TODO: At least on Preview that is. Where does this come from exactly?
const MIN_LOVELACE_PER_TRANSACTION: u64 = 840_450;

const CREDIT_POLL_INTERVAL: Duration = Duration::from_secs(1);

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
                * config.microtransactions_per_fanout
                + MIN_LOVELACE_PER_TRANSACTION) as f64
            / 1_000_000.0;
        if config.commit_ada < minimal_commit {
            Err(anyhow!(
                "hydras-manager: Please make sure that configured commit_ada ≥ lovelace_per_request * requests_per_microtransaction * microtransactions_per_fanout + {}.",
                MIN_LOVELACE_PER_TRANSACTION as f64 / 1_000_000.0
            ))?
        }

        let microtransaction_lovelace: u64 =
            config.lovelace_per_request * config.requests_per_microtransaction;
        if microtransaction_lovelace < MIN_LOVELACE_PER_TRANSACTION {
            Err(anyhow!(
                "hydras-manager: Please make sure that each microtransaction will be larger than {MIN_LOVELACE_PER_TRANSACTION} lovelace. Currently it would be {microtransaction_lovelace}."
            ))?
        }

        Ok(Self {
            config: HydraConfig::load(config.clone(), network).await?,
            controller_counter: Arc::new(Arc::new(())),
        })
    }

    pub async fn initialize_key_exchange(
        &self,
        req: KeyExchangeRequest,
    ) -> Result<KeyExchangeResponse> {
        if req.accepted_platform_h2h_port.is_some() {
            Err(anyhow!(
                "`accepted_platform_h2h_port` must not be set in `initialize_key_exchange`"
            ))?
        }

        let cur_count = Arc::strong_count(self.controller_counter.as_ref()).saturating_sub(1);
        if cur_count as u64 >= self.config.toml.max_concurrent_hydra_nodes {
            let err = anyhow!(
                "Too many concurrent `hydra-node`s already running. You can increase the limit in config."
            );
            warn!("{err}");
            Err(err)?
        }

        let have_funds: f64 = self
            .config
            .lovelace_on_addr(&self.config.gateway_cardano_addr)
            .await? as f64
            / 1_000_000.0;
        let required_funds_ada: f64 = MIN_FUEL_LOVELACE as f64 / 1_000_000.0;
        if have_funds < required_funds_ada {
            let err = anyhow!(
                "hydra-controller: {} ADA is too little for the Hydra L1 fees on the enterprise address associated with {:?}. Please provide at least {} ADA",
                have_funds,
                self.config.toml.cardano_signing_key,
                required_funds_ada,
            );
            error!("{err}");
            Err(err)?
        }
        info!(
            "hydra-controller: funds on cardano_signing_key: {:?} ADA",
            have_funds
        );

        use verifications::{find_free_tcp_port, read_json_file};

        let config_dir = mk_config_dir(&self.config.network, &req.machine_id)?;
        self.config.gen_hydra_keys(&config_dir).await?;

        Ok(KeyExchangeResponse {
            machine_id: verifications::hashed_machine_id(),
            gateway_cardano_vkey: self.config.gateway_cardano_vkey.clone(),
            gateway_hydra_vkey: read_json_file(&config_dir.join("hydra.vk"))?,
            hydra_scripts_tx_id: hydra_scripts_tx_id(&self.config.network).to_string(),
            protocol_parameters: self.config.protocol_parameters.clone(),
            contestation_period: CONTESTATION_PERIOD_SECONDS,
            proposed_platform_h2h_port: find_free_tcp_port().await?,
            gateway_h2h_port: find_free_tcp_port().await?,
            kex_done: false,
            commit_ada: self.config.toml.commit_ada,
            lovelace_per_request: self.config.toml.lovelace_per_request,
            requests_per_microtransaction: self.config.toml.requests_per_microtransaction,
            microtransactions_per_fanout: self.config.toml.microtransactions_per_fanout,
        })
    }

    /// You should first call [`Self::initialize_key_exchange`], and then this
    /// function with the initial request/response pair.
    pub async fn spawn_new(
        &self,
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
                "The Bridge must accept the same port that was proposed to it."
            ))?
        }

        // Clone first, to prevent the nastier race condition:
        let maybe_new = Arc::clone(self.controller_counter.as_ref());
        let new_count = Arc::strong_count(self.controller_counter.as_ref()).saturating_sub(1);
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
            final_req.machine_id.clone(),
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

/// Runs a `hydra-node` and sets up an L2 network with the Bridge for microtransactions.
///
/// You can safely clone it, and the clone will represent the same `hydra-node` etc.
#[derive(Clone)]
pub struct HydraController {
    event_tx: mpsc::Sender<Event>,
    credits_available: Arc<AtomicU64>,
    head_open: Arc<AtomicBool>,
    _controller_counter: Arc<()>,
}

#[derive(Debug)]
pub enum CreditError {
    HeadNotOpen,
    InsufficientCredits,
}

impl std::fmt::Display for CreditError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CreditError::HeadNotOpen => write!(f, "hydra head is not open"),
            CreditError::InsufficientCredits => write!(f, "insufficient prepaid credits"),
        }
    }
}

impl std::error::Error for CreditError {}

// FIXME: send a Quit event on `drop()` of all controller instances

#[derive(serde::Deserialize, serde::Serialize, Debug, PartialEq, Eq, Clone)]
pub struct KeyExchangeRequest {
    pub machine_id: String,
    pub platform_cardano_vkey: serde_json::Value,
    pub platform_hydra_vkey: serde_json::Value,
    pub accepted_platform_h2h_port: Option<u16>,
}

#[derive(serde::Deserialize, serde::Serialize, Debug, PartialEq, Clone)]
pub struct KeyExchangeResponse {
    pub machine_id: String,
    pub gateway_cardano_vkey: serde_json::Value,
    pub gateway_hydra_vkey: serde_json::Value,
    pub hydra_scripts_tx_id: String,
    pub protocol_parameters: serde_json::Value,
    pub contestation_period: Duration,
    /// Unfortunately the ports have to be the same on both sides, so
    /// since we’re tunneling through the WebSocket, and our hosts are
    /// both 127.0.0.1, the Gateway has to propose the port on the
    /// Bridge, too (as both sides open both ports).
    pub proposed_platform_h2h_port: u16,
    pub gateway_h2h_port: u16,
    /// This being set to `true` means that the ceremony is successful, and the
    /// Gateway is going to start its own `hydra-node`, and the Bridge should too.
    pub kex_done: bool,
    pub commit_ada: f64,
    pub lovelace_per_request: u64,
    pub requests_per_microtransaction: u64,
    pub microtransactions_per_fanout: u64,
}

impl HydraController {
    async fn spawn(
        config: HydraConfig,
        customer_id: String,
        controller_counter: Arc<()>,
        kex_req: KeyExchangeRequest,
        kex_resp: KeyExchangeResponse,
    ) -> Result<Self> {
        let credits_available = Arc::new(AtomicU64::new(0));
        let head_open = Arc::new(AtomicBool::new(false));
        let event_tx = State::spawn(
            config,
            customer_id.clone(),
            kex_req,
            kex_resp,
            credits_available.clone(),
            head_open.clone(),
        )
        .await?;
        Ok(Self {
            event_tx,
            credits_available,
            head_open,
            _controller_counter: controller_counter,
        })
    }

    // FIXME: this is too primitive
    pub fn is_alive(&self) -> bool {
        !self.event_tx.is_closed()
    }

    pub fn try_consume_credit(&self) -> Result<(), CreditError> {
        if !self.head_open.load(Ordering::SeqCst) {
            return Err(CreditError::HeadNotOpen);
        }

        let mut current = self.credits_available.load(Ordering::SeqCst);
        loop {
            if current == 0 {
                return Err(CreditError::InsufficientCredits);
            }

            match self.credits_available.compare_exchange(
                current,
                current - 1,
                Ordering::SeqCst,
                Ordering::SeqCst,
            ) {
                Ok(_) => return Ok(()),
                Err(next) => current = next,
            }
        }
    }

    pub async fn terminate(&self) {
        let _ = self.event_tx.send(Event::Terminate).await;
    }
}

enum Event {
    Restart,
    Terminate,
    TryToInitHead,
    TryToCommit,
    WaitForOpen,
    MonitorCredits,
    TryToClose,
    WaitForClosed { retries_before_reclose: u64 },
    DoFanout,
    WaitForIdleAfterClose,
}

fn mk_config_dir(network: &Network, customer_machine_id: &str) -> Result<PathBuf> {
    let config_dir = dirs::config_dir()
        .ok_or(anyhow!("`dirs::config_dir()` returned `None`"))?
        .join("blockfrost-gateway")
        .join("hydra")
        .join(network.as_str())
        .join(format!("customer-{customer_machine_id}"));
    std::fs::create_dir_all(&config_dir)?;
    Ok(config_dir)
}

// FIXME: don’t construct all key and other paths manually, keep them in a single place
struct State {
    config: HydraConfig,
    customer_log_id: String,
    config_dir: PathBuf,
    event_tx: mpsc::Sender<Event>,
    kex_req: KeyExchangeRequest,
    kex_resp: KeyExchangeResponse,
    api_port: u16,
    metrics_port: u16,
    hydra_peers_connected: bool,
    hydra_head_open: bool,
    credits_available: Arc<AtomicU64>,
    head_open_flag: Arc<AtomicBool>,
    credits_last_balance: u64,
    received_microtransactions: u64,
    is_closing: bool,
    hydra_pid: Option<u32>,
}

impl State {
    const RESTART_DELAY: Duration = Duration::from_secs(5);

    async fn spawn(
        config: HydraConfig,
        customer_id: String,
        kex_req: KeyExchangeRequest,
        kex_resp: KeyExchangeResponse,
        credits_available: Arc<AtomicU64>,
        head_open_flag: Arc<AtomicBool>,
    ) -> Result<mpsc::Sender<Event>> {
        let config_dir = mk_config_dir(&config.network, &customer_id)?;
        let customer_log_id = format!("customer-{customer_id}");

        let (event_tx, mut event_rx) = mpsc::channel::<Event>(32);

        let mut self_ = Self {
            config,
            customer_log_id,
            config_dir,
            event_tx: event_tx.clone(),
            kex_req,
            kex_resp,
            api_port: 0,
            metrics_port: 0,
            hydra_peers_connected: false,
            hydra_head_open: false,
            credits_available,
            head_open_flag,
            credits_last_balance: 0,
            received_microtransactions: 0,
            is_closing: false,
            hydra_pid: None,
        };

        self_.send(Event::Restart).await;

        tokio::spawn(async move {
            while let Some(event) = event_rx.recv().await {
                match self_.process_event(event).await {
                    Ok(()) => (),
                    Err(err) => {
                        error!(
                            "hydra-controller: {}: error: {}; will restart in {:?}…",
                            self_.customer_log_id,
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
                info!("hydra-controller: {}: starting…", self.customer_log_id);
                self.hydra_head_open = false;
                self.head_open_flag.store(false, Ordering::SeqCst);
                self.credits_available.store(0, Ordering::SeqCst);
                self.credits_last_balance = 0;
                self.received_microtransactions = 0;
                self.is_closing = false;
                self.start_hydra_node().await?;
                self.send_delayed(Event::TryToInitHead, Duration::from_secs(1))
                    .await
            },

            Event::Terminate => {
                if let Some(pid) = self.hydra_pid {
                    verifications::sigterm(pid)?
                }
            },

            Event::TryToInitHead => {
                let ready = verifications::prometheus_metric_at_least(
                    &format!("http://127.0.0.1:{}/metrics", self.metrics_port),
                    "hydra_head_peers_connected",
                    1.0,
                )
                .await;

                info!(
                    "hydra-controller: {}: waiting for hydras to connect: ready={:?}",
                    self.customer_log_id, ready
                );

                if matches!(ready, Ok(true)) {
                    self.hydra_peers_connected = true;

                    verifications::send_one_websocket_msg(
                        &format!("ws://127.0.0.1:{}/", self.api_port),
                        serde_json::json!({"tag":"Init"}),
                        Duration::from_secs(2),
                    )
                    .await?;

                    self.send_delayed(Event::TryToCommit, Duration::from_secs(3))
                        .await
                } else {
                    self.send_delayed(Event::TryToInitHead, Duration::from_secs(1))
                        .await
                }
            },

            Event::TryToCommit => {
                let status = verifications::fetch_head_tag(self.api_port).await;

                info!(
                    "hydra-controller: {}: waiting for the Initial head status: status={:?}",
                    self.customer_log_id, status
                );

                match status.as_deref() {
                    Err(_) => {
                        self.send_delayed(Event::TryToCommit, Duration::from_secs(3))
                            .await
                    },
                    Ok(status) => {
                        if status == "Initial" {
                            info!(
                                "hydra-controller: {}: submitting an empty Commit transaction to join the Hydra Head",
                                self.customer_log_id
                            );
                            self.config
                                .empty_commit_to_hydra(
                                    self.api_port,
                                    &self.config.toml.cardano_signing_key,
                                )
                                .await?;
                            self.send_delayed(Event::WaitForOpen, Duration::from_secs(3))
                                .await
                        } else if status == "Open" {
                            self.send_delayed(Event::WaitForOpen, Duration::from_secs(3))
                                .await
                        } else {
                            self.send_delayed(Event::TryToCommit, Duration::from_secs(3))
                                .await
                        }
                    },
                }
            },

            Event::WaitForOpen => {
                let status = verifications::fetch_head_tag(self.api_port).await?;
                info!(
                    "hydra-controller: {}: waiting for the Open head status: status={:?}",
                    self.customer_log_id, status
                );
                if status == "Open" {
                    self.hydra_head_open = true;
                    self.head_open_flag.store(true, Ordering::SeqCst);
                    self.credits_available.store(0, Ordering::SeqCst);
                    self.credits_last_balance = 0;
                    self.received_microtransactions = 0;
                    self.send_delayed(Event::MonitorCredits, CREDIT_POLL_INTERVAL)
                        .await;
                } else {
                    self.send_delayed(Event::WaitForOpen, Duration::from_secs(3))
                        .await
                }
            },

            Event::MonitorCredits => {
                if self.hydra_head_open && !self.is_closing {
                    match verifications::lovelace_in_snapshot_for_address(
                        self.api_port,
                        &self.config.gateway_cardano_addr,
                    )
                    .await
                    {
                        Ok(current_balance) => {
                            if current_balance < self.credits_last_balance {
                                warn!(
                                    "hydra-controller: {}: snapshot balance decreased ({} -> {}), resetting",
                                    self.customer_log_id,
                                    self.credits_last_balance,
                                    current_balance
                                );
                                self.credits_last_balance = current_balance;
                            } else {
                                let delta = current_balance - self.credits_last_balance;
                                if delta > 0 {
                                    let microtransaction_lovelace =
                                        self.config.toml.lovelace_per_request
                                            * self.config.toml.requests_per_microtransaction;
                                    if microtransaction_lovelace == 0 {
                                        warn!(
                                            "hydra-controller: {}: microtransaction value is zero; ignoring credits",
                                            self.customer_log_id
                                        );
                                    } else if delta >= microtransaction_lovelace {
                                        let new_microtransactions =
                                            delta / microtransaction_lovelace;
                                        let new_credits = new_microtransactions
                                            * self.config.toml.requests_per_microtransaction;
                                        self.credits_available
                                            .fetch_add(new_credits, Ordering::SeqCst);
                                        self.received_microtransactions += new_microtransactions;
                                        info!(
                                            "hydra-controller: {}: received {} microtransaction(s), req. credits +{}",
                                            self.customer_log_id,
                                            new_microtransactions,
                                            new_credits
                                        );
                                    } else {
                                        warn!(
                                            "hydra-controller: {}: snapshot delta {} is below expected microtransaction size {}",
                                            self.customer_log_id, delta, microtransaction_lovelace
                                        );
                                    }
                                    self.credits_last_balance = current_balance;
                                }
                            }

                            if self.received_microtransactions
                                >= self.config.toml.microtransactions_per_fanout
                                && !self.is_closing
                            {
                                self.is_closing = true;
                                self.send_delayed(Event::TryToClose, Duration::from_secs(1))
                                    .await;
                            }
                        },
                        Err(err) => warn!(
                            "hydra-controller: {}: failed to read snapshot/utxo: {err}",
                            self.customer_log_id
                        ),
                    }
                    self.send_delayed(Event::MonitorCredits, CREDIT_POLL_INTERVAL)
                        .await;
                }
            },

            Event::TryToClose => {
                self.hydra_head_open = false;
                self.head_open_flag.store(false, Ordering::SeqCst);
                verifications::send_one_websocket_msg(
                    &format!("ws://127.0.0.1:{}", self.api_port),
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
                    "hydra-controller: {}: waiting for the Closed head status: status={:?}",
                    self.customer_log_id, status
                );
                if status == "Closed" {
                    let invalidity_period = (2 + 1) * CONTESTATION_PERIOD_SECONDS;
                    info!(
                        "hydra-controller: {}: will wait through the invalidity period ({:?}) before requesting `Fanout`",
                        self.customer_log_id, invalidity_period,
                    );
                    self.send_delayed(Event::DoFanout, invalidity_period).await
                } else {
                    self.send_delayed(
                        if retries_before_reclose <= 1 {
                            Event::TryToClose
                        } else {
                            Event::WaitForClosed {
                                retries_before_reclose: retries_before_reclose - 1,
                            }
                        },
                        Duration::from_secs(3),
                    )
                    .await
                }
            },

            Event::DoFanout => {
                info!(
                    "hydra-controller: {}: requesting `Fanout`",
                    self.customer_log_id,
                );
                verifications::send_one_websocket_msg(
                    &format!("ws://127.0.0.1:{}", self.api_port),
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
                    "hydra-controller: {}: waiting for the Idle head status (after Fanout): status={:?}",
                    self.customer_log_id, status
                );
                if status == "Idle" {
                    info!(
                        "hydra-controller: {}: re-initializing the Hydra Head for another L2 session",
                        self.customer_log_id,
                    );

                    self.is_closing = false;
                    self.received_microtransactions = 0;
                    self.credits_last_balance = 0;
                    self.send_delayed(Event::TryToInitHead, Duration::from_secs(3))
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
            .arg("gateway-node")
            .arg("--persistence-dir")
            .arg(self.config_dir.join("persistence"))
            .arg("--cardano-signing-key")
            .arg(&self.config.toml.cardano_signing_key)
            .arg("--hydra-signing-key")
            .arg(self.config_dir.join("hydra.sk"))
            .arg("--hydra-scripts-tx-id")
            .arg(&self.kex_resp.hydra_scripts_tx_id)
            .arg("--ledger-protocol-parameters")
            .arg(&protocol_parameters_path)
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
            .arg(format!(
                "127.0.0.1:{}",
                self.kex_resp.proposed_platform_h2h_port
            ))
            .arg("--monitoring-port")
            .arg(format!("{}", self.metrics_port))
            .arg("--hydra-verification-key")
            .arg(platform_hydra_vkey_path)
            .arg("--cardano-verification-key")
            .arg(platform_cardano_vkey_path)
            .stdin(Stdio::null())
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
