use anyhow::{Context, Result, anyhow, bail};
use bf_common::cardano_keys;
use pallas_network::facades::NodeClient;
use pallas_network::miniprotocols::{
    localstate::queries_v16,
    localtxsubmission::{EraTx, Response},
};
use std::path::Path;
use tracing::info;

impl super::State {
    /// Generates Hydra keys if they don’t exist.
    pub(super) async fn gen_hydra_keys(&self) -> Result<()> {
        std::fs::create_dir_all(&self.config_dir)?;

        let key_path = self.config_dir.join("hydra.sk");

        if !key_path.exists() {
            info!("generating hydra keys");

            let status = tokio::process::Command::new(&self.hydra_node_exe)
                .arg("gen-hydra-key")
                .arg("--output-file")
                .arg(self.config_dir.join("hydra"))
                .status()
                .await?;

            if !status.success() {
                bail!("gen-hydra-key failed with status: {status}");
            }
        } else {
            info!("hydra keys already exist");
        }

        Ok(())
    }

    /// Check how much lovelace is on an enterprise address associated with a
    /// given `payment.skey`, by querying the local `cardano-node` via the
    /// Ouroboros local-state-query mini-protocol (Pallas).
    pub(super) async fn lovelace_on_payment_skey(&self, skey_path: &Path) -> Result<u64> {
        let bech32_addr =
            cardano_keys::derive_enterprise_address(skey_path, self.network.as_str())?;

        let address = pallas_addresses::Address::from_bech32(&bech32_addr)
            .map_err(|e| anyhow!("invalid bech32 address: {e}"))?;
        let addr_bytes: pallas_codec::utils::Bytes = address.to_vec().into();
        let addrs: Vec<pallas_codec::utils::Bytes> = vec![addr_bytes];

        let magic = self.genesis.network_magic as u64;
        let mut client = NodeClient::connect(&self.node_socket_path, magic)
            .await
            .map_err(|e| anyhow!("failed to connect to cardano-node: {e}"))?;

        let statequery = client.statequery();
        statequery
            .acquire(None)
            .await
            .map_err(|e| anyhow!("failed to acquire statequery: {e}"))?;

        let era = queries_v16::get_current_era(statequery)
            .await
            .map_err(|e| anyhow!("get_current_era failed: {e}"))?;

        let utxos = queries_v16::get_utxo_by_address(statequery, era, addrs)
            .await
            .map_err(|e| anyhow!("get_utxo_by_address failed: {e}"))?;

        statequery
            .send_release()
            .await
            .map_err(|e| anyhow!("statequery release failed: {e}"))?;

        let total: u64 = utxos
            .iter()
            .map(|(_, output)| {
                let value = match output {
                    queries_v16::TransactionOutput::Current(o) => &o.amount,
                    queries_v16::TransactionOutput::Legacy(o) => &o.amount,
                };
                match value {
                    queries_v16::Value::Coin(coin) => u64::from(coin),
                    queries_v16::Value::Multiasset(coin, _) => u64::from(coin),
                }
            })
            .sum();

        Ok(total)
    }

    /// Derive a verification-key JSON envelope from a signing-key file.
    pub(super) fn derive_vkey_from_skey(skey_path: &Path) -> Result<serde_json::Value> {
        cardano_keys::derive_vkey_from_skey(skey_path)
    }

    /// POST an empty commit to the Hydra head, sign the resulting transaction
    /// with CSL, and submit it to the local `cardano-node` via the Ouroboros
    /// local-tx-submission mini-protocol (Pallas).
    pub(super) async fn empty_commit_to_hydra(
        &self,
        hydra_api_port: u16,
        signing_skey: &Path,
    ) -> Result<()> {
        use reqwest::header;

        // POST an empty commit to get an unsigned transaction envelope
        let url = format!("http://127.0.0.1:{hydra_api_port}/commit");
        let client = reqwest::Client::new();
        let resp = client
            .post(url)
            .header(header::CONTENT_TYPE, "application/json")
            .body("{}")
            .send()
            .await
            .context("failed to POST /commit to hydra-node")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.bytes().await.unwrap_or_default();
            bail!(
                "hydra /commit failed with {}: {}",
                status,
                String::from_utf8_lossy(&body)
            );
        }

        let commit_tx_envelope: serde_json::Value = resp
            .json()
            .await
            .context("hydra /commit response was not valid JSON")?;

        // Sign the transaction using CSL
        let signed_envelope = cardano_keys::sign_tx_envelope(&commit_tx_envelope, signing_skey)?;
        let signed_cbor_hex = signed_envelope
            .get("cborHex")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("signed tx envelope missing cborHex"))?;
        let signed_cbor = hex::decode(signed_cbor_hex)?;

        // Submit via Pallas local-tx-submission to the local cardano-node
        let magic = self.genesis.network_magic as u64;
        let mut node = NodeClient::connect(&self.node_socket_path, magic)
            .await
            .map_err(|e| anyhow!("failed to connect to cardano-node for tx submit: {e}"))?;

        // Query current era (needed for EraTx wrapper)
        let statequery = node.statequery();
        statequery
            .acquire(None)
            .await
            .map_err(|e| anyhow!("statequery acquire failed: {e}"))?;

        let era = queries_v16::get_current_era(statequery)
            .await
            .map_err(|e| anyhow!("get_current_era failed: {e}"))?;

        statequery
            .send_release()
            .await
            .map_err(|e| anyhow!("statequery release failed: {e}"))?;

        // Submit the signed transaction
        let submission = node.submission();
        let era_tx = EraTx(era, signed_cbor);
        match submission.submit_tx(era_tx).await {
            Ok(Response::Accepted) => {
                info!("commit transaction accepted by cardano-node");
                Ok(())
            },
            Ok(Response::Rejected(reason)) => {
                bail!("commit transaction rejected by cardano-node: {reason:?}")
            },
            Err(e) => bail!("error submitting commit transaction to cardano-node: {e}"),
        }
    }
}

/// Reads a JSON file from disk.
pub fn read_json_file(path: &Path) -> Result<serde_json::Value> {
    let contents = std::fs::read_to_string(path)?;
    let json: serde_json::Value = serde_json::from_str(&contents)?;
    Ok(json)
}

/// Writes `json` to `path` (pretty-printed) **only if** the JSON content differs
/// from what is already on disk. Returns `true` if the file was written.
pub fn write_json_if_changed(path: &Path, json: &serde_json::Value) -> Result<bool> {
    use std::fs::File;
    use std::io::Write;

    if path.exists() {
        if let Ok(existing_str) = std::fs::read_to_string(path) {
            if let Ok(existing_json) = serde_json::from_str::<serde_json::Value>(&existing_str) {
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

/// Finds a free port by binding to port 0, to let the OS pick a free port.
pub async fn find_free_tcp_port() -> std::io::Result<u16> {
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0)).await?;
    let port = listener.local_addr()?.port();
    drop(listener);
    Ok(port)
}

/// Returns `Ok(true)` if `port` can be bound on 127.0.0.1 (so it's free),
/// `Ok(false)` if it's already in use, and `Err(_)` for other IO errors.
pub async fn is_tcp_port_free(port: u16) -> std::io::Result<bool> {
    match tokio::net::TcpListener::bind(("127.0.0.1", port)).await {
        Ok(listener) => {
            drop(listener);
            Ok(true)
        },
        Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => Ok(false),
        Err(e) => Err(e),
    }
}

pub async fn fetch_head_tag(hydra_api_port: u16) -> Result<String> {
    let url = format!("http://127.0.0.1:{hydra_api_port}/head");

    let v: serde_json::Value = reqwest::get(url).await?.error_for_status()?.json().await?;

    v.get("tag")
        .ok_or(anyhow!("missing tag"))
        .and_then(|a| a.as_str().ok_or(anyhow!("tag is not a string")))
        .map(|a| a.to_string())
}
