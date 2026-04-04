use crate::types::Network;
use anyhow::{Result, anyhow, bail};
use std::path::PathBuf;
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

pub mod verifications;

const MIN_FUEL_LOVELACE: u64 = 15_000_000;
const MIN_COMMIT_TOPUP_LOVELACE: u64 = 1_000_000;
const CREDIT_POLL_INTERVAL: Duration = Duration::from_secs(1);

#[derive(Clone, Debug)]
pub struct HydraConfig {
    pub cardano_signing_key: PathBuf,
    pub blockfrost_project_id: String,
    pub network: Network,
}

/// Runs a `hydra-node` and sets up an L2 network with the Gateway for microtransactions.
///
/// You can safely clone it, and the clone will represent the same `hydra-node` etc.
#[derive(Clone)]
pub struct HydraController {
    event_tx: mpsc::Sender<Event>,
    credits_available: Arc<AtomicU64>,
}

#[derive(Debug)]
pub enum CreditError {
    InsufficientCredits,
}

impl std::fmt::Display for CreditError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
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
        let event_tx = State::spawn(
            config,
            kex_requests,
            kex_responses,
            terminate_reqs,
            credits_available.clone(),
        )
        .await?;
        Ok(Self {
            event_tx,
            credits_available,
        })
    }

    pub fn try_reserve_credit(&self) -> Result<(), CreditError> {
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
            .unwrap_or_else(|_| error!("failed to account one request: event channel closed"))
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
    blockfrost_api: Option<blockfrost::BlockfrostAPI>,
    /// Shared HTTP client for all outgoing requests.
    http: reqwest::Client,
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
    head_open_initialized: bool,
    credits_available: Arc<AtomicU64>,
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
    ) -> Result<mpsc::Sender<Event>> {
        let hydra_node_exe =
            bf_common::find_libexec::find_libexec("hydra-node", "HYDRA_NODE_PATH", &["--version"])
                .map_err(|e| anyhow!(e))?;

        let config_dir = dirs::config_dir()
            .ok_or_else(|| {
                anyhow!(
                    "Could not determine config directory (HOME or XDG_CONFIG_HOME may be unset)"
                )
            })?
            .join("blockfrost-sdk-bridge")
            .join("hydra")
            .join(config.network.as_str())
            .join("_default");

        let (event_tx, mut event_rx) = mpsc::channel::<Event>(32);

        let platform_cardano_vkey =
            bf_common::cardano_keys::derive_vkey_from_skey(&config.cardano_signing_key)?;

        let blockfrost_api = blockfrost::BlockfrostAPI::new(
            &config.blockfrost_project_id,
            blockfrost::BlockFrostSettings::default(),
        );

        let mut self_ = Self {
            config,
            hydra_node_exe,
            blockfrost_api: Some(blockfrost_api),
            http: reqwest::Client::new(),
            config_dir,
            platform_cardano_vkey,
            gateway_payment_addr: String::new(),
            payment_params: None,
            event_tx: event_tx.clone(),
            kex_requests,
            api_port: 0,
            metrics_port: 0,
            last_hydra_head_state: String::new(),
            hydra_pid: None,
            hydra_head_open: false,
            head_open_initialized: false,
            credits_available,
            credits_last_balance: 0,
            accounted_requests: 0,
            sent_microtransactions: 0,
            commit_wallet_skey: PathBuf::new(),
            commit_wallet_addr: String::new(),
            prepay_sent: false,
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
                        error!("error: {}; will restart in {:?}…", err, Self::RESTART_DELAY);
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
            event_tx
                .send(event)
                .await
                .expect("we never close the event receiver");
        });
    }

    fn blockfrost_api(&self) -> Result<&blockfrost::BlockfrostAPI> {
        self.blockfrost_api
            .as_ref()
            .ok_or_else(|| anyhow!("blockfrost API not initialized"))
    }

    async fn process_event(&mut self, event: Event) -> Result<()> {
        match event {
            Event::Restart => {
                info!("starting…");

                let potential_fuel = self
                    .lovelace_on_payment_skey(&self.config.cardano_signing_key)
                    .await?;
                if potential_fuel < MIN_FUEL_LOVELACE {
                    bail!(
                        "{} ADA is too little for the Hydra L1 fees on the enterprise address associated with {:?}. Please provide at least {} ADA",
                        potential_fuel as f64 / 1_000_000.0,
                        self.config.cardano_signing_key,
                        MIN_FUEL_LOVELACE as f64 / 1_000_000.0,
                    )
                }

                info!("fuel on cardano_signing_key: {:?} lovelace", potential_fuel);

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
                self.credits_available.store(0, Ordering::SeqCst);
                self.credits_last_balance = 0;
                self.accounted_requests = 0;
                self.sent_microtransactions = 0;
                self.prepay_sent = false;
                self.head_open_initialized = false;
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
                    "payment params commit_ada={} lovelace_per_request={} requests_per_microtransaction={} microtransactions_per_fanout={}",
                    params.commit_ada,
                    params.lovelace_per_request,
                    params.requests_per_microtransaction,
                    params.microtransactions_per_fanout
                );
                self.payment_params = Some(params);

                if self.gateway_payment_addr.is_empty() {
                    let addr = self
                        .derive_enterprise_address_from_vkey_json(&kex_resp.gateway_cardano_vkey)?;
                    info!("gateway payment address: {}", addr);
                    self.gateway_payment_addr = addr;
                }

                if !(matches!(
                    verifications::is_tcp_port_free(kex_resp.gateway_h2h_port).await,
                    Ok(true)
                ) && matches!(
                    verifications::is_tcp_port_free(kex_resp.proposed_platform_h2h_port).await,
                    Ok(true)
                )) {
                    warn!("the ports proposed by the Gateway are not free locally, will ask again");
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
                        .derive_enterprise_address_from_vkey_json(&kex_resp.gateway_cardano_vkey)?;
                    info!("gateway payment address: {}", addr);
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
                // Fund the commit wallet *before* `Init` so that the fund tx
                // and hydra-node's `Init` tx don't race for the same
                // signing-key UTxOs.
                self.send_delayed(Event::FundCommitAddr, Duration::from_secs(1))
                    .await
            },

            Event::FundCommitAddr => {
                let commit_wallet = self.config_dir.join("commit-funds");
                self.commit_wallet_skey = commit_wallet.with_extension("sk");

                if !self.commit_wallet_skey.exists() {
                    Self::new_cardano_keypair(&commit_wallet)?;
                }

                self.commit_wallet_addr =
                    self.derive_enterprise_address_from_skey(&self.commit_wallet_skey)?;

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
                        "topping up commit address by {} lovelace (current={}, target={})",
                        top_up, current_lovelace, target_lovelace
                    );
                    self.fund_address(
                        &self.derive_enterprise_address_from_skey(
                            &self.config.cardano_signing_key,
                        )?,
                        &self.commit_wallet_addr,
                        top_up,
                        &self.config.cardano_signing_key,
                    )
                    .await?;
                } else {
                    info!(
                        "commit address already funded (current={}, target={})",
                        current_lovelace, target_lovelace
                    );
                }

                // The Bridge never sends `Init`, as it waits for the Gateway's
                // `Init` to land. `TryToCommit` polls the head status and retries
                // until the head is "Initial".
                self.send_delayed(Event::TryToCommit, Duration::from_secs(3))
                    .await
            },

            Event::TryToCommit => {
                // Check head status first – the Gateway sends `Init`,
                // the Bridge just waits for it to appear on L1.
                let status = verifications::fetch_head_tag(&self.http, self.api_port).await;

                info!("waiting for the Initial head status: status={:?}", status);

                match status.as_deref() {
                    Err(_) => {
                        self.send_delayed(Event::TryToCommit, Duration::from_secs(3))
                            .await
                    },
                    Ok("Open") => {
                        info!("head already Open, skipping commit");
                        self.send_delayed(Event::WaitForOpen, Duration::from_secs(1))
                            .await
                    },
                    Ok("Initial") => {
                        let commit_wallet_lovelace = self
                            .lovelace_on_payment_skey(&self.commit_wallet_skey)
                            .await?;

                        let params = self
                            .payment_params
                            .clone()
                            .ok_or(anyhow!("payment parameters not set before commit"))?;

                        let lovelace_needed = 0.99 * params.commit_ada * 1_000_000.0;

                        info!(
                            "commit address lovelace={}, needed={}",
                            commit_wallet_lovelace,
                            lovelace_needed.round()
                        );

                        if commit_wallet_lovelace as f64 >= lovelace_needed {
                            info!("submitting a Commit transaction to join the Hydra Head");
                            match self
                                .commit_all_utxo_to_hydra(
                                    &self.commit_wallet_addr,
                                    self.api_port,
                                    &self.commit_wallet_skey,
                                )
                                .await
                            {
                                Ok(()) => {
                                    self.send_delayed(Event::WaitForOpen, Duration::from_secs(3))
                                        .await
                                },
                                Err(err) => {
                                    warn!("commit failed (will retry): {err}");
                                    self.send_delayed(Event::TryToCommit, Duration::from_secs(30))
                                        .await
                                },
                            }
                        } else {
                            self.send_delayed(Event::TryToCommit, Duration::from_secs(3))
                                .await
                        }
                    },
                    Ok(_) => {
                        // Head is in some other state (`Idle`, `Closed`, etc.),
                        // let’s keep polling until the Gateway's `Init` lands.
                        self.send_delayed(Event::TryToCommit, Duration::from_secs(3))
                            .await
                    },
                }
            },

            Event::WaitForOpen => {
                let status = verifications::fetch_head_tag(&self.http, self.api_port).await?;
                info!("waiting for the Open head status: status={:?}", status);
                if status == "Open" {
                    self.last_hydra_head_state = status.clone();
                    self.send_delayed(Event::MonitorStates, Duration::from_secs(5))
                        .await;
                    self.on_head_open().await?;
                } else {
                    self.send_delayed(Event::WaitForOpen, Duration::from_secs(3))
                        .await
                }
            },

            Event::MonitorStates => {
                let new_status = verifications::fetch_head_tag(&self.http, self.api_port).await?;

                if new_status != self.last_hydra_head_state {
                    let old = self.last_hydra_head_state.clone();
                    let new = new_status.clone();
                    self.last_hydra_head_state = new_status.clone();

                    info!("state changed from {old} to {new}");

                    if new == "Initial" {
                        self.send_delayed(Event::FundCommitAddr, Duration::from_secs(1))
                            .await;
                    }
                }

                if new_status == "Open" {
                    self.on_head_open().await?;
                } else {
                    self.hydra_head_open = false;
                    self.credits_last_balance = 0;
                    self.head_open_initialized = false;
                }

                self.send_delayed(Event::MonitorStates, Duration::from_secs(5))
                    .await;
            },

            Event::MonitorCredits => {
                if self.hydra_head_open {
                    debug!(
                        "MonitorCredits: credits={}, last_balance={}, sent_microtxs={}, accounted_reqs={}",
                        self.credits_available.load(Ordering::SeqCst),
                        self.credits_last_balance,
                        self.sent_microtransactions,
                        self.accounted_requests,
                    );
                    if self.gateway_payment_addr.is_empty() {
                        warn!("gateway payment address not set yet");
                    } else if let Some(params) = &self.payment_params {
                        match verifications::lovelace_in_snapshot_for_address(
                            &self.http,
                            self.api_port,
                            &self.gateway_payment_addr,
                        )
                        .await
                        {
                            Ok(current_balance) => {
                                if current_balance < self.credits_last_balance {
                                    warn!(
                                        "snapshot balance decreased ({} -> {}), resetting",
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
                                                "microtransaction value is zero; ignoring credits"
                                            );
                                        } else if delta >= microtransaction_lovelace {
                                            let new_microtransactions =
                                                delta / microtransaction_lovelace;
                                            let new_credits = new_microtransactions
                                                * params.requests_per_microtransaction;
                                            self.credits_available
                                                .fetch_add(new_credits, Ordering::SeqCst);
                                            info!(
                                                "req. credits +{} ({} microtransaction(s))",
                                                new_credits, new_microtransactions
                                            );
                                        } else {
                                            warn!(
                                                "snapshot delta {} is below expected microtransaction size {}",
                                                delta, microtransaction_lovelace
                                            );
                                        }
                                        self.credits_last_balance = current_balance;
                                    }
                                }
                            },
                            Err(err) => {
                                warn!("failed to read snapshot/utxo: {err}")
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
                        warn!("payment parameters not set yet");
                        return Ok(());
                    },
                };

                if !self.hydra_head_open {
                    warn!(
                        "request not yet accounted because Hydra Head is not Open; retrying shortly"
                    );
                    self.send_delayed(Event::AccountOneRequest, Duration::from_millis(500))
                        .await;
                    return Ok(());
                }

                if self.gateway_payment_addr.is_empty() {
                    warn!("gateway payment address not set yet");
                    return Ok(());
                }

                self.accounted_requests += 1;

                if self.accounted_requests >= params.requests_per_microtransaction {
                    info!("sending a microtransaction");
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
            warn!("gateway payment address not set yet");
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

    async fn on_head_open(&mut self) -> Result<()> {
        if self.head_open_initialized {
            self.hydra_head_open = true;
            return Ok(());
        }

        self.head_open_initialized = true;
        self.hydra_head_open = true;
        self.credits_last_balance = 0;
        self.accounted_requests = 0;
        self.sent_microtransactions = 0;
        self.prepay_sent = false;
        self.send_delayed(Event::MonitorCredits, CREDIT_POLL_INTERVAL)
            .await;

        // Wait before sending the prepay microtransaction. Both hydra-nodes
        // must be in "Open" state for the snapshot to be signed. There can be a
        // delay of tens of seconds between the Bridge and Gateway observing
        // "Open" (Blockfrost lag). Without this delay the prepay tx may get
        // `TxValid` but never reach `SnapshotConfirmed` because the Gateway's node
        // wasn't ready to co-sign.
        info!("delaying prepay by 15 s to let both nodes settle into Open");
        tokio::time::sleep(Duration::from_secs(15)).await;

        self.send_prepay_microtransaction().await?;
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

        // Write the Blockfrost project ID to a file for hydra-node's --blockfrost option
        let blockfrost_project_id_path = self.config_dir.join("blockfrost-project-id");
        std::fs::write(
            &blockfrost_project_id_path,
            &self.config.blockfrost_project_id,
        )?;

        let mut cmd = tokio::process::Command::new(&self.hydra_node_exe);
        cmd.arg("--node-id")
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
            .arg("--blockfrost")
            .arg(&blockfrost_project_id_path)
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
            .stderr(Stdio::piped());

        // Ask the kernel to `SIGTERM` `hydra-node` if our process dies (e.g.
        // killed by a test harness). This only applies to this single `cmd`.
        #[cfg(target_os = "linux")]
        unsafe {
            cmd.pre_exec(|| {
                nix::libc::prctl(nix::libc::PR_SET_PDEATHSIG, nix::libc::SIGTERM);
                Ok(())
            });
        }

        let mut child = cmd.spawn()?;

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
