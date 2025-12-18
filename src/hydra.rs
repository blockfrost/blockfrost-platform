use crate::config::HydraConfig as HydraTomlConfig;
use crate::types::{AssetName, Network};
use anyhow::{Result, anyhow};
use serde::Deserialize;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

pub mod verifications;

// FIXME: this should most probably be back to the default of 600 seconds:
const CONTESTATION_PERIOD_SECONDS: std::time::Duration = std::time::Duration::from_secs(60);

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
        Ok(Self {
            config: HydraConfig::load(config.clone(), network).await?,
            controller_counter: Arc::new(Arc::new(())),
        })
    }

    pub async fn initialize_key_exchange(
        &self,
        _originator: &AssetName,
        req: KeyExchangeRequest,
    ) -> Result<KeyExchangeResponse> {
        if req.accepted_platform_h2h_port.is_some() {
            Err(anyhow!(
                "`accepted_platform_h2h_port` must not be set in `initialize_key_exchange`"
            ))?
        }

        // FIXME: actually exchange
        use verifications::{find_free_tcp_port, read_json_file};

        // TODO: save protocol params
        //
        // std::fs::create_dir_all(&self.config_dir)?;
        // let pp_path = self.config_dir.join("protocol-parameters.json");
        // if write_json_if_changed(&pp_path, &params)? {
        //     info!("hydra-controller: protocol parameters updated");
        // } else {
        //     info!("hydra-controller: protocol parameters unchanged");
        // }

        let resp = KeyExchangeResponse {
            gateway_cardano_vkey: read_json_file(
                "/home/mw/.config/blockfrost-platform/hydra/tmp_their_keys/payment.vk".as_ref(),
            )?,
            gateway_hydra_vkey: read_json_file(
                "/home/mw/.config/blockfrost-platform/hydra/tmp_their_keys/hydra.vk".as_ref(),
            )?,
            hydra_scripts_tx_id: hydra_scripts_tx_id(&self.config.network).to_string(),
            protocol_parameters: read_json_file(
                "/home/mw/.config/blockfrost-platform/hydra/tmp_their_keys/protocol-parameters.json"
                    .as_ref(),
            )?,
            contestation_period: CONTESTATION_PERIOD_SECONDS,
            proposed_platform_h2h_port: find_free_tcp_port().await?,
            gateway_h2h_port: find_free_tcp_port().await?,
        };
        Ok(resp)
    }

    /// You should first call [`Self::initialize_key_exchange`], and then this
    /// function with the initial request/response pair.
    pub async fn spawn_new(
        &self,
        originator: &AssetName,
        initial: (KeyExchangeRequest, KeyExchangeResponse),
        final_req: KeyExchangeRequest,
    ) -> Result<(HydraController, KeyExchangeResponse)> {
        // Clone first, to prevent the nastier race condition:
        let maybe_new = Arc::clone(self.controller_counter.as_ref());
        let new_count = Arc::strong_count(&self.controller_counter).saturating_sub(1); // subtract the manager
        if new_count as u64 <= self.config.toml.max_concurrent_hydra_nodes {
            Err(anyhow!(
                "Too many concurrent `hydra-node`s already running. You can increase the limit in config."
            ))?
        } else {
            // FIXME: continue
            let final_resp = initial.1.clone();

            let ctl = HydraController::spawn(
                self.config.clone(),
                originator.clone(),
                maybe_new,
                final_req,
                final_resp.clone(),
            )
            .await?;
            // FIXME:
            Ok((ctl, final_resp))
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
struct HydraConfig {
    pub toml: HydraTomlConfig,
    pub network: Network,
    pub hydra_node_exe: String,
    pub cardano_cli_exe: String,
    pub gateway_cardano_vkey: serde_json::Value,
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
        };
        let gateway_cardano_vkey = self_
            .derive_vkey_from_skey(&self_.toml.cardano_signing_key)
            .await?;
        let self_ = Self {
            gateway_cardano_vkey,
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
    pub contestation_period: std::time::Duration,
    /// Unfortunately the ports have to be the same on both sides, so
    /// since we’re tunneling through the WebSocket, and our hosts are
    /// both 127.0.0.1, the Gateway has to propose the port on the
    /// Platform, too (as both sides open both ports).
    pub proposed_platform_h2h_port: u16,
    pub gateway_h2h_port: u16,
}

impl HydraController {
    async fn spawn(
        config: HydraConfig,
        originator: AssetName,
        controller_counter: Arc<()>,
        kex_req: KeyExchangeRequest,
        kex_resp: KeyExchangeResponse,
    ) -> Result<Self> {
        let event_tx = State::spawn(config, originator, kex_req, kex_resp).await?;
        Ok(Self {
            event_tx,
            _controller_counter: controller_counter,
        })
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
    KeyExchangeResponse(KeyExchangeResponse),
    SomeEvent { _some_value: u64 },
}

fn mk_config_dir(network: &Network, originator: &AssetName) -> Result<PathBuf> {
    let config_dir = dirs::config_dir()
        .ok_or(anyhow!("`dirs::config_dir()` returned `None`"))?
        .join("blockfrost-platform")
        .join("hydra")
        .join(network.as_str())
        .join(originator.as_str());
    std::fs::create_dir_all(&config_dir)?;
    Ok(config_dir)
}

// FIXME: don’t construct all key and other paths manually, keep them in a single place
struct State {
    config: HydraConfig,
    _originator: AssetName,
    platform_cardano_vkey: serde_json::Value,
    config_dir: PathBuf,
    event_tx: mpsc::Sender<Event>,
}

impl State {
    const RESTART_DELAY: std::time::Duration = std::time::Duration::from_secs(5);
    // FIXME: shouldn’t this be multiplied by `max_concurrent_hydra_nodes`?
    const MIN_FUEL_LOVELACE: u64 = 15_000_000;

    async fn spawn(
        config: HydraConfig,
        originator: AssetName,
        kex_req: KeyExchangeRequest,
        kex_resp: KeyExchangeResponse,
    ) -> Result<mpsc::Sender<Event>> {
        let config_dir = mk_config_dir(&config.network, &originator)?;

        let (event_tx, mut event_rx) = mpsc::channel::<Event>(32);

        let self_ = Self {
            config,
            _originator: originator,
            platform_cardano_vkey: kex_req.platform_cardano_vkey,
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

    async fn process_event(&self, event: Event) -> Result<()> {
        match event {
            Event::Restart => {
                info!("hydra-controller: starting…");

                let potential_fuel = self
                    .config
                    .lovelace_on_payment_skey(&self.config.toml.cardano_signing_key)
                    .await?;
                if potential_fuel < Self::MIN_FUEL_LOVELACE {
                    Err(anyhow!(
                        "hydra-controller: {} ADA is too little for the Hydra L1 fees on the enterprise address associated with {:?}. Please provide at least {} ADA",
                        potential_fuel as f64 / 1_000_000.0,
                        self.config.toml.cardano_signing_key,
                        Self::MIN_FUEL_LOVELACE as f64 / 1_000_000.0,
                    ))?
                }

                info!(
                    "hydra-controller: fuel on cardano_signing_key: {:?} lovelace",
                    potential_fuel
                );

                self.config.gen_hydra_keys(&self.config_dir).await?;

                self.start_hydra_node(todo!()).await?;
            },

            Event::KeyExchangeResponse(kex_resp) => {},

            Event::SomeEvent { .. } => todo!(),
        }
        Ok(())
    }

    async fn start_hydra_node(&self, kex_response: KeyExchangeResponse) -> Result<()> {
        use std::process::Stdio;
        use tokio::io::{AsyncBufReadExt, BufReader};

        // FIXME: save the ports in an `Arc<Mutex<u16>` for future use
        let api_port = verifications::find_free_tcp_port().await?;
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
            .arg(&kex_response.hydra_scripts_tx_id)
            .arg("--ledger-protocol-parameters")
            .arg(&protocol_parameters_path) // FIXME: copy it somewhere else in case the source file changes
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
            .arg(&self.config.toml.node_socket_path)
            .arg("--api-port")
            .arg(format!("{api_port}"))
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
