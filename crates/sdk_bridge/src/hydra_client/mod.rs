use crate::types::Network;
use anyhow::{Result, anyhow};
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

const MIN_FUEL_LOVELACE: u64 = 15_000_000;
const MIN_COMMIT_TOPUP_LOVELACE: u64 = 1_000_000;
const CREDIT_POLL_INTERVAL: Duration = Duration::from_secs(1);

#[derive(Clone, Debug)]
pub struct HydraConfig {
    pub cardano_signing_key: PathBuf,
    pub node_socket_path: PathBuf,
    pub network: Network,
}

/// Runs a `hydra-node` and sets up an L2 network with the Gateway for microtransactions.
///
/// You can safely clone it, and the clone will represent the same `hydra-node` etc.
#[derive(Clone)]
pub struct HydraController {
    event_tx: mpsc::Sender<Event>,
    credits_available: Arc<AtomicU64>,
    head_open: Arc<AtomicBool>,
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
    pub contestation_period: std::time::Duration,
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

pub struct TerminateRequest;

impl HydraController {
    #[allow(clippy::too_many_arguments)]
    pub async fn spawn(
        config: HydraConfig,
        kex_requests: mpsc::Sender<KeyExchangeRequest>,
        kex_responses: mpsc::Receiver<KeyExchangeResponse>,
        terminate_reqs: mpsc::Receiver<TerminateRequest>,
    ) -> Result<Self> {
        let credits_available = Arc::new(AtomicU64::new(0));
        let head_open = Arc::new(AtomicBool::new(false));
        let event_tx = State::spawn(
            config,
            kex_requests,
            kex_responses,
            terminate_reqs,
            credits_available.clone(),
            head_open.clone(),
        )
        .await?;
        Ok(Self {
            event_tx,
            credits_available,
            head_open,
        })
    }

    pub fn try_reserve_credit(&self) -> Result<(), CreditError> {
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

    pub async fn account_one_request(&self) {
        self.event_tx
            .send(Event::AccountOneRequest)
            .await
            .unwrap_or_else(|_| {
                error!("hydra-controller: failed to account one request: event channel closed")
            })
    }
}

#[derive(Clone, Debug)]
struct PaymentParams {
    commit_ada: f64,
    lovelace_per_request: u64,
    requests_per_microtransaction: u64,
    microtransactions_per_fanout: u64,
}

enum Event {
    Restart,
    Terminate,
    KeyExchangeResponse(KeyExchangeResponse),
    TryToInitHead,
    FundCommitAddr,
    TryToCommit,
    WaitForOpen,
    MonitorStates,
    AccountOneRequest,
    MonitorCredits,
}

// FIXME: don’t construct all key and other paths manually, keep them in a single place
struct State {
    config: HydraConfig,
    hydra_node_exe: String,
    cardano_cli_exe: String,
    config_dir: PathBuf,
    platform_cardano_vkey: serde_json::Value,
    gateway_payment_addr: String,
    payment_params: Option<PaymentParams>,
    event_tx: mpsc::Sender<Event>,
    kex_requests: mpsc::Sender<KeyExchangeRequest>,
    api_port: u16,
    metrics_port: u16,
    last_hydra_head_state: String,
    hydra_pid: Option<u32>,
    hydra_head_open: bool,
    credits_available: Arc<AtomicU64>,
    head_open_flag: Arc<AtomicBool>,
    credits_last_balance: u64,
    accounted_requests: u64,
    sent_microtransactions: u64,
    commit_wallet_skey: PathBuf,
    commit_wallet_addr: String,
    prepay_sent: bool,
}

impl State {
    const RESTART_DELAY: Duration = Duration::from_secs(5);

    #[allow(clippy::too_many_arguments)]
    async fn spawn(
        config: HydraConfig,
        kex_requests: mpsc::Sender<KeyExchangeRequest>,
        kex_responses: mpsc::Receiver<KeyExchangeResponse>,
        terminate_reqs: mpsc::Receiver<TerminateRequest>,
        credits_available: Arc<AtomicU64>,
        head_open_flag: Arc<AtomicBool>,
    ) -> Result<mpsc::Sender<Event>> {
        let hydra_node_exe =
            crate::find_libexec::find_libexec("hydra-node", "HYDRA_NODE_PATH", &["--version"])
                .map_err(|e| anyhow!(e))?;
        let cardano_cli_exe =
            crate::find_libexec::find_libexec("cardano-cli", "CARDANO_CLI_PATH", &["version"])
                .map_err(|e| anyhow!(e))?;

        let config_dir = dirs::config_dir()
            .expect("Could not determine config directory")
            .join("blockfrost-sdk-bridge")
            .join("hydra")
            .join(config.network.as_str())
            .join("_default");

        let (event_tx, mut event_rx) = mpsc::channel::<Event>(32);

        let mut self_ = Self {
            config,
            hydra_node_exe,
            cardano_cli_exe,
            config_dir,
            platform_cardano_vkey: serde_json::Value::Null,
            gateway_payment_addr: String::new(),
            payment_params: None,
            event_tx: event_tx.clone(),
            kex_requests,
            api_port: 0,
            metrics_port: 0,
            last_hydra_head_state: String::new(),
            hydra_pid: None,
            hydra_head_open: false,
            credits_available,
            head_open_flag,
            credits_last_balance: 0,
            accounted_requests: 0,
            sent_microtransactions: 0,
            commit_wallet_skey: PathBuf::new(),
            commit_wallet_addr: String::new(),
            prepay_sent: false,
        };

        let platform_cardano_vkey = self_
            .derive_vkey_from_skey(&self_.config.cardano_signing_key)
            .await?;
        self_.platform_cardano_vkey = platform_cardano_vkey;

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
                if potential_fuel < MIN_FUEL_LOVELACE {
                    Err(anyhow!(
                        "hydra-controller: {} ADA is too little for the Hydra L1 fees on the enterprise address associated with {:?}. Please provide at least {} ADA",
                        potential_fuel as f64 / 1_000_000.0,
                        self.config.cardano_signing_key,
                        MIN_FUEL_LOVELACE as f64 / 1_000_000.0,
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
                self.head_open_flag.store(false, Ordering::SeqCst);
                self.credits_available.store(0, Ordering::SeqCst);
                self.credits_last_balance = 0;
                self.accounted_requests = 0;
                self.sent_microtransactions = 0;
                self.prepay_sent = false;
                self.last_hydra_head_state = String::new();
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
                let params = PaymentParams {
                    commit_ada: kex_resp.commit_ada,
                    lovelace_per_request: kex_resp.lovelace_per_request,
                    requests_per_microtransaction: kex_resp.requests_per_microtransaction,
                    microtransactions_per_fanout: kex_resp.microtransactions_per_fanout,
                };
                info!(
                    "hydra-controller: payment params commit_ada={} lovelace_per_request={} requests_per_microtransaction={} microtransactions_per_fanout={}",
                    params.commit_ada,
                    params.lovelace_per_request,
                    params.requests_per_microtransaction,
                    params.microtransactions_per_fanout
                );
                self.payment_params = Some(params);

                if self.gateway_payment_addr.is_empty() {
                    let addr = self
                        .derive_enterprise_address_from_vkey_json(&kex_resp.gateway_cardano_vkey)
                        .await?;
                    info!("hydra-controller: gateway payment address: {}", addr);
                    self.gateway_payment_addr = addr;
                }

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
                if self.gateway_payment_addr.is_empty() {
                    let addr = self
                        .derive_enterprise_address_from_vkey_json(&kex_resp.gateway_cardano_vkey)
                        .await?;
                    info!("hydra-controller: gateway payment address: {}", addr);
                    self.gateway_payment_addr = addr;
                }

                if self.payment_params.is_none() {
                    self.payment_params = Some(PaymentParams {
                        commit_ada: kex_resp.commit_ada,
                        lovelace_per_request: kex_resp.lovelace_per_request,
                        requests_per_microtransaction: kex_resp.requests_per_microtransaction,
                        microtransactions_per_fanout: kex_resp.microtransactions_per_fanout,
                    });
                }

                self.start_hydra_node(kex_resp).await?;
                self.send_delayed(Event::TryToInitHead, Duration::from_secs(1))
                    .await
            },

            Event::TryToInitHead => {
                let ready = verifications::prometheus_metric_at_least(
                    &format!("http://127.0.0.1:{}/metrics", self.metrics_port),
                    "hydra_head_peers_connected",
                    1.0,
                )
                .await;

                info!(
                    "hydra-controller: waiting for hydras to connect: ready={:?}",
                    ready
                );

                if matches!(ready, Ok(true)) {
                    verifications::send_one_websocket_msg(
                        &format!("ws://127.0.0.1:{}/", self.api_port),
                        serde_json::json!({"tag":"Init"}),
                        Duration::from_secs(2),
                    )
                    .await?;

                    self.send_delayed(Event::FundCommitAddr, Duration::from_secs(3))
                        .await
                } else {
                    self.send_delayed(Event::TryToInitHead, Duration::from_secs(1))
                        .await
                }
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

                    if !self.commit_wallet_skey.exists() {
                        if status == "Open" {
                            Err(anyhow!(
                                "Head status is Open, but there’s no commit wallet anymore; this shouldn’t really happen"
                            ))?
                        }

                        self.new_cardano_keypair(&commit_wallet).await?;
                    }

                    self.commit_wallet_addr = self
                        .derive_enterprise_address_from_skey(&self.commit_wallet_skey)
                        .await?;

                    if status == "Initial" {
                        let params = self.payment_params.clone().ok_or(anyhow!(
                            "payment parameters not set before funding commit address"
                        ))?;

                        let target_lovelace = (params.commit_ada * 1_000_000.0).round() as u64;
                        let current_lovelace = self
                            .lovelace_on_payment_skey(&self.commit_wallet_skey)
                            .await?;

                        if current_lovelace < target_lovelace {
                            let mut top_up = target_lovelace - current_lovelace;
                            if top_up < MIN_COMMIT_TOPUP_LOVELACE {
                                top_up = MIN_COMMIT_TOPUP_LOVELACE;
                            }
                            info!(
                                "hydra-controller: topping up commit address by {} lovelace (current={}, target={})",
                                top_up, current_lovelace, target_lovelace
                            );
                            self.fund_address(
                                &self
                                    .derive_enterprise_address_from_skey(
                                        &self.config.cardano_signing_key,
                                    )
                                    .await?,
                                &self.commit_wallet_addr,
                                top_up,
                                &self.config.cardano_signing_key,
                            )
                            .await?;
                        } else {
                            info!(
                                "hydra-controller: commit address already funded (current={}, target={})",
                                current_lovelace, target_lovelace
                            );
                        }

                        self.send_delayed(Event::TryToCommit, Duration::from_secs(3))
                            .await
                    } else if status == "Open" {
                        warn!(
                            "hydra-controller: turns out the Head is already Open, skipping Commit"
                        );
                        self.send_delayed(Event::WaitForOpen, Duration::from_secs(3))
                            .await
                    }
                } else {
                    self.send_delayed(Event::FundCommitAddr, Duration::from_secs(3))
                        .await
                }
            },

            Event::TryToCommit => {
                let commit_wallet_lovelace = self
                    .lovelace_on_payment_skey(&self.commit_wallet_skey)
                    .await?;

                let params = self
                    .payment_params
                    .clone()
                    .ok_or(anyhow!("payment parameters not set before commit"))?;

                let lovelace_needed = 0.99 * params.commit_ada * 1_000_000.0;

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
                        .await
                } else {
                    self.send_delayed(Event::TryToCommit, Duration::from_secs(3))
                        .await
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
                    self.head_open_flag.store(true, Ordering::SeqCst);
                    self.credits_available.store(0, Ordering::SeqCst);
                    self.credits_last_balance = 0;
                    self.prepay_sent = false;
                    self.last_hydra_head_state = status.clone();
                    self.send_delayed(Event::MonitorCredits, CREDIT_POLL_INTERVAL)
                        .await;
                    self.send_delayed(Event::MonitorStates, Duration::from_secs(5))
                        .await;
                    self.send_prepay_microtransaction().await?;
                } else {
                    self.send_delayed(Event::WaitForOpen, Duration::from_secs(3))
                        .await
                }
            },

            Event::MonitorStates => {
                let new_status = verifications::fetch_head_tag(self.api_port).await?;

                if new_status != self.last_hydra_head_state {
                    let old = self.last_hydra_head_state.clone();
                    let new = new_status.clone();
                    self.last_hydra_head_state = new_status.clone();

                    info!("hydra-controller: state changed from {old} to {new}");

                    if new == "Initial" {
                        self.send_delayed(Event::FundCommitAddr, Duration::from_secs(1))
                            .await;
                    }
                }

                if new_status == "Open" {
                    self.hydra_head_open = true;
                    self.head_open_flag.store(true, Ordering::SeqCst);
                } else {
                    self.hydra_head_open = false;
                    self.head_open_flag.store(false, Ordering::SeqCst);
                    self.credits_available.store(0, Ordering::SeqCst);
                    self.credits_last_balance = 0;
                }

                self.send_delayed(Event::MonitorStates, Duration::from_secs(5))
                    .await;
            },

            Event::MonitorCredits => {
                if self.hydra_head_open {
                    if self.gateway_payment_addr.is_empty() {
                        warn!("hydra-controller: gateway payment address not set yet");
                    } else if let Some(params) = &self.payment_params {
                        match verifications::lovelace_in_snapshot_for_address(
                            self.api_port,
                            &self.gateway_payment_addr,
                        )
                        .await
                        {
                            Ok(current_balance) => {
                                if current_balance < self.credits_last_balance {
                                    warn!(
                                        "hydra-controller: snapshot balance decreased ({} -> {}), resetting",
                                        self.credits_last_balance, current_balance
                                    );
                                    self.credits_last_balance = current_balance;
                                } else {
                                    let delta = current_balance - self.credits_last_balance;
                                    if delta > 0 {
                                        let microtransaction_lovelace = params.lovelace_per_request
                                            * params.requests_per_microtransaction;
                                        if microtransaction_lovelace == 0 {
                                            warn!(
                                                "hydra-controller: microtransaction value is zero; ignoring credits"
                                            );
                                        } else if delta >= microtransaction_lovelace {
                                            let new_microtransactions =
                                                delta / microtransaction_lovelace;
                                            let new_credits = new_microtransactions
                                                * params.requests_per_microtransaction;
                                            self.credits_available
                                                .fetch_add(new_credits, Ordering::SeqCst);
                                            info!(
                                                "hydra-controller: req. credits +{} ({} microtransaction(s))",
                                                new_credits, new_microtransactions
                                            );
                                        } else {
                                            warn!(
                                                "hydra-controller: snapshot delta {} is below expected microtransaction size {}",
                                                delta, microtransaction_lovelace
                                            );
                                        }
                                        self.credits_last_balance = current_balance;
                                    }
                                }
                            },
                            Err(err) => {
                                warn!("hydra-controller: failed to read snapshot/utxo: {err}")
                            },
                        }
                    }

                    self.send_delayed(Event::MonitorCredits, CREDIT_POLL_INTERVAL)
                        .await;
                }
            },

            Event::AccountOneRequest => {
                let params = match &self.payment_params {
                    Some(p) => p.clone(),
                    None => {
                        warn!("hydra-controller: payment parameters not set yet");
                        return Ok(());
                    },
                };

                if !self.hydra_head_open {
                    warn!(
                        "hydra-controller: would account a request, but the Hydra Head is not Open"
                    );
                    return Ok(());
                }

                if self.gateway_payment_addr.is_empty() {
                    warn!("hydra-controller: gateway payment address not set yet");
                    return Ok(());
                }

                self.accounted_requests += 1;

                if self.accounted_requests >= params.requests_per_microtransaction {
                    info!("hydra-controller: sending a microtransaction");
                    let amount_lovelace: u64 =
                        self.accounted_requests * params.lovelace_per_request;
                    self.send_hydra_transaction(
                        self.api_port,
                        &self.commit_wallet_addr,
                        &self.gateway_payment_addr,
                        &self.commit_wallet_skey,
                        amount_lovelace,
                    )
                    .await?;

                    self.accounted_requests = 0;
                    self.sent_microtransactions += 1;
                }
            },
        }
        Ok(())
    }

    async fn send_prepay_microtransaction(&mut self) -> Result<()> {
        if self.prepay_sent {
            return Ok(());
        }

        let params = self
            .payment_params
            .clone()
            .ok_or(anyhow!("payment parameters not set before prepay"))?;

        if self.gateway_payment_addr.is_empty() {
            warn!("hydra-controller: gateway payment address not set yet");
            return Ok(());
        }

        let amount_lovelace: u64 =
            params.requests_per_microtransaction * params.lovelace_per_request;
        self.send_hydra_transaction(
            self.api_port,
            &self.commit_wallet_addr,
            &self.gateway_payment_addr,
            &self.commit_wallet_skey,
            amount_lovelace,
        )
        .await?;

        self.sent_microtransactions += 1;
        self.prepay_sent = true;
        Ok(())
    }

    async fn start_hydra_node(&mut self, kex_response: KeyExchangeResponse) -> Result<()> {
        use std::process::Stdio;
        use tokio::io::{AsyncBufReadExt, BufReader};

        self.api_port = verifications::find_free_tcp_port().await?;
        self.metrics_port = verifications::find_free_tcp_port().await?;

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
            .arg("bridge-node")
            .arg("--persistence-dir")
            .arg(self.config_dir.join("persistence"))
            .arg("--cardano-signing-key")
            .arg(&self.config.cardano_signing_key)
            .arg("--hydra-signing-key")
            .arg(self.config_dir.join("hydra.sk"))
            .arg("--hydra-scripts-tx-id")
            .arg(&kex_response.hydra_scripts_tx_id)
            .arg("--ledger-protocol-parameters")
            .arg(&protocol_parameters_path)
            .arg("--contestation-period")
            .arg(format!("{}s", kex_response.contestation_period.as_secs()))
            .args(if self.config.network == Network::Mainnet {
                vec!["-mainnet".to_string()]
            } else {
                vec![
                    "--testnet-magic".to_string(),
                    format!("{}", self.config.network.network_magic()),
                ]
            })
            .arg("--node-socket")
            .arg(&self.config.node_socket_path)
            .arg("--api-port")
            .arg(format!("{}", self.api_port))
            .arg("--api-host")
            .arg("127.0.0.1")
            .arg("--listen")
            .arg(format!(
                "127.0.0.1:{}",
                kex_response.proposed_platform_h2h_port
            ))
            .arg("--peer")
            .arg(format!("127.0.0.1:{}", kex_response.gateway_h2h_port))
            .arg("--monitoring-port")
            .arg(format!("{}", self.metrics_port))
            .arg("--hydra-verification-key")
            .arg(gateway_hydra_vkey_path)
            .arg("--cardano-verification-key")
            .arg(gateway_cardano_vkey_path)
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
