use anyhow::{Result, anyhow, bail};
use blockfrost::Pagination;
use serde_json::Value;
use std::path::Path;
use tracing::info;

use bf_common::cardano_keys;

/// FIXME: proper errors, not `anyhow!`
impl super::HydraConfig {
    /// Generates Hydra keys if they don’t exist.
    pub(super) async fn gen_hydra_keys(&self, target_dir: &Path) -> Result<()> {
        std::fs::create_dir_all(target_dir)?;

        let key_path = target_dir.join("hydra.sk");

        if !key_path.exists() {
            info!("generating hydra keys");

            let status = tokio::process::Command::new(&self.hydra_node_exe)
                .arg("gen-hydra-key")
                .arg("--output-file")
                .arg(target_dir.join("hydra"))
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

    /// Fetch L1 protocol parameters via Blockfrost, convert them from
    /// the Blockfrost API format (snake_case) to the cardano-cli format
    /// (camelCase) expected by hydra-node's `--ledger-protocol-parameters`,
    /// and zero out tx fees for use as Hydra L2 ledger parameters.
    pub(super) async fn gen_protocol_parameters(&self) -> Result<serde_json::Value> {
        let params = self.blockfrost_api.epochs_latest_parameters().await?;
        let bf = serde_json::to_value(&params)?;

        let mut json = blockfrost_params_to_shelley(&bf)?;

        if let Some(obj) = json.as_object_mut() {
            obj.insert("txFeeFixed".to_string(), 0.into());
            obj.insert("txFeePerByte".to_string(), 0.into());

            if let Some(exec_prices) = obj
                .get_mut("executionUnitPrices")
                .and_then(Value::as_object_mut)
            {
                exec_prices.insert("priceMemory".to_string(), 0.into());
                exec_prices.insert("priceSteps".to_string(), 0.into());
            }
        }

        Ok(json)
    }

    /// Check how much lovelace is available on an address (via Blockfrost).
    pub(super) async fn lovelace_on_addr(&self, address: &str) -> Result<u64> {
        let utxos = self
            .blockfrost_api
            .addresses_utxos(address, Pagination::all())
            .await;

        match utxos {
            Ok(utxos) => Ok(cardano_keys::sum_lovelace_from_blockfrost_utxos(&utxos)),
            Err(e) => {
                // Blockfrost returns 404 if address has never been seen
                let msg = e.to_string();
                if msg.contains("404") {
                    Ok(0)
                } else {
                    bail!("blockfrost addresses_utxos failed: {e}")
                }
            },
        }
    }

    /// Derive a verification-key JSON envelope from a signing-key file.
    pub(super) fn derive_vkey_from_skey(skey_path: &Path) -> Result<serde_json::Value> {
        cardano_keys::derive_vkey_from_skey(skey_path)
    }

    /// Derive an enterprise (payment-only) bech32 address from a signing-key.
    pub(super) fn derive_enterprise_address_from_skey(&self, skey_path: &Path) -> Result<String> {
        cardano_keys::derive_enterprise_address(skey_path, self.network.as_str())
    }

    /// Commit with an empty UTxO set to a Hydra Head via the
    /// hydra-node `/commit` endpoint. Signs and submits the resulting L1
    /// transaction using CSL + Blockfrost.
    pub(super) async fn empty_commit_to_hydra(
        &self,
        hydra_api_port: u16,
        signing_skey: &Path,
    ) -> Result<()> {
        use anyhow::Context;
        use reqwest::header;

        let url = format!("http://127.0.0.1:{hydra_api_port}/commit");
        let resp = self
            .http
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

        let commit_tx_bytes = resp
            .bytes()
            .await
            .context("failed to read hydra /commit response body")?
            .to_vec();

        // Sign with CSL
        let tx_envelope: serde_json::Value = serde_json::from_slice(&commit_tx_bytes)
            .context("hydra /commit response was not valid JSON")?;

        let signed_tx = cardano_keys::sign_tx_envelope(&tx_envelope, signing_skey)?;
        let signed_cbor_hex = signed_tx["cborHex"]
            .as_str()
            .ok_or_else(|| anyhow!("signed tx missing cborHex"))?;
        let signed_cbor = hex::decode(signed_cbor_hex)?;

        // Submit via Blockfrost
        self.blockfrost_api
            .transactions_submit(signed_cbor.to_vec())
            .await?;

        Ok(())
    }
}

pub async fn lovelace_in_snapshot_for_address(hydra_api_port: u16, address: &str) -> Result<u64> {
    use anyhow::Context;

    let snapshot_url = format!("http://127.0.0.1:{hydra_api_port}/snapshot/utxo");
    let utxo: Value = reqwest::Client::new()
        .get(&snapshot_url)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await
        .context("snapshot/utxo: failed to decode JSON")?;

    let utxo_obj = utxo
        .as_object()
        .context("snapshot/utxo: expected top-level JSON object")?;

    let mut filtered: serde_json::Map<String, Value> = serde_json::Map::new();
    for (k, v) in utxo_obj.iter() {
        if v.get("address").and_then(Value::as_str) == Some(address) {
            filtered.insert(k.clone(), v.clone());
        }
    }

    let filtered_json = Value::Object(filtered);
    cardano_keys::sum_lovelace_from_utxo_json(&filtered_json)
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

    if path.exists()
        && let Ok(existing_str) = std::fs::read_to_string(path)
        && let Ok(existing_json) = serde_json::from_str::<serde_json::Value>(&existing_str)
        && existing_json == *json
    {
        return Ok(false);
    }

    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)?;
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

/// Checks if a Prometheus `metric` at `url` is greater or equal to `threshold`.
pub async fn prometheus_metric_at_least(url: &str, metric: &str, threshold: f64) -> Result<bool> {
    let client = reqwest::Client::new();
    let body = client
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;

    let mut found_any = false;
    let mut max_value: Option<f64> = None;

    for line in body.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // <name>{labels} <value> [timestamp]
        let mut parts = line.split_whitespace();
        let name_and_labels = match parts.next() {
            Some(x) => x,
            None => continue,
        };

        let name = name_and_labels.split('{').next().unwrap_or(name_and_labels);
        if name != metric {
            continue;
        }

        let value_str = match parts.next() {
            Some(v) => v,
            None => continue,
        };

        let value: f64 = value_str.parse()?;
        found_any = true;
        max_value = Some(max_value.map_or(value, |m| m.max(value)));
    }

    if !found_any {
        bail!("metric {metric} not found in /metrics output");
    }

    Ok(max_value.unwrap_or(f64::NEG_INFINITY) >= threshold)
}

/// Sends a single WebSocket message, and waits a bit before closing the
/// connection cleanly. Particularly useful for Hydra.
pub async fn send_one_websocket_msg(
    url: &str,
    payload: serde_json::Value,
    wait_before_close: std::time::Duration,
) -> Result<()> {
    use futures_util::{SinkExt, StreamExt};
    use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

    let (ws_stream, _resp) = connect_async(url).await?;
    let (mut write, mut read) = ws_stream.split();

    write
        .send(Message::Text(payload.to_string().into()))
        .await?;

    tokio::time::sleep(wait_before_close).await;

    write.send(Message::Close(None)).await?;

    // Drain until we observe the close handshake (or the peer drops).
    // Use a timeout so we never block the caller indefinitely — the hydra-node
    // may keep the WebSocket open and continue sending events even after the
    // client sends a Close frame.
    let _ = tokio::time::timeout(std::time::Duration::from_secs(4), async {
        while let Some(msg) = read.next().await {
            match msg {
                Ok(Message::Close(_)) => break,
                Ok(Message::Text(msg)) => {
                    tracing::info!("got WebSocket message: {}", msg)
                },
                Ok(msg) => tracing::info!("got WebSocket message: {:?}", msg),
                Err(_) => break,
            }
        }
    })
    .await;

    Ok(())
}

pub async fn fetch_head_tag(hydra_api_port: u16) -> Result<String> {
    let url = format!("http://127.0.0.1:{hydra_api_port}/head");

    let v: serde_json::Value = reqwest::get(url).await?.error_for_status()?.json().await?;

    v.get("tag")
        .ok_or(anyhow!("missing tag"))
        .and_then(|a| a.as_str().ok_or(anyhow!("tag is not a string")))
        .map(|a| a.to_string())
}

/// Returns `true` when the Hydra head is `Closed` **and** `readyToFanoutSent`
/// is `true`, meaning the contestation deadline has passed on-chain and a
/// Fanout transaction can be theoretically submitted (it may still sometimes
/// fail due to slot-lag — we retry then).
pub async fn fetch_head_ready_to_fanout(hydra_api_port: u16) -> Result<bool> {
    let url = format!("http://127.0.0.1:{hydra_api_port}/head");

    let v: serde_json::Value = reqwest::get(url).await?.error_for_status()?.json().await?;

    let tag = v.get("tag").and_then(|t| t.as_str()).unwrap_or_default();
    let ready = v
        .pointer("/contents/readyToFanoutSent")
        .and_then(|r| r.as_bool())
        .unwrap_or(false);

    Ok(tag == "Closed" && ready)
}

/// Convert Blockfrost epoch-parameters JSON (snake_case field names) to the
/// cardano-cli protocol-parameters format (camelCase) that hydra-node expects
/// for `--ledger-protocol-parameters`.
///
/// Field mapping follows the reference implementation at
/// <https://github.com/blockfrost/blockfrost-cardano-cli/blob/master/src/commands/query/protocol-parameters.ts>
/// and the output of `cardano-cli query protocol-parameters` (Conway era).
fn blockfrost_params_to_shelley(bf: &serde_json::Value) -> Result<serde_json::Value> {
    let obj = bf
        .as_object()
        .ok_or_else(|| anyhow!("expected JSON object for epoch parameters"))?;

    /// Parse a string-or-number value into a JSON integer.
    fn as_int(v: Option<&Value>) -> Value {
        match v {
            Some(Value::String(s)) => s.parse::<u64>().map_or(Value::Null, Value::from),
            Some(n @ Value::Number(_)) => n.clone(),
            _ => Value::Null,
        }
    }

    fn get(obj: &serde_json::Map<String, Value>, key: &str) -> Value {
        obj.get(key).cloned().unwrap_or(Value::Null)
    }

    // Prefer `cost_models_raw` (integer arrays, same as cardano-cli output)
    // over `cost_models` (named parameter maps with divergent key names).
    let cost_models = obj
        .get("cost_models_raw")
        .filter(|v| !v.is_null())
        .or_else(|| obj.get("cost_models").filter(|v| !v.is_null()))
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));

    let mut result = serde_json::json!({
        // --- Shelley-era fields ---
        "txFeePerByte": get(obj, "min_fee_a"),
        "txFeeFixed": get(obj, "min_fee_b"),
        "maxBlockBodySize": get(obj, "max_block_size"),
        "maxTxSize": get(obj, "max_tx_size"),
        "maxBlockHeaderSize": get(obj, "max_block_header_size"),
        "stakeAddressDeposit": as_int(obj.get("key_deposit")),
        "stakePoolDeposit": as_int(obj.get("pool_deposit")),
        "poolRetireMaxEpoch": get(obj, "e_max"),
        "stakePoolTargetNum": get(obj, "n_opt"),
        "poolPledgeInfluence": get(obj, "a0"),
        "monetaryExpansion": get(obj, "rho"),
        "treasuryCut": get(obj, "tau"),
        "protocolVersion": {
            "major": get(obj, "protocol_major_ver"),
            "minor": get(obj, "protocol_minor_ver"),
        },
        "minPoolCost": as_int(obj.get("min_pool_cost")),
        // --- Alonzo-era fields ---
        "executionUnitPrices": {
            "priceMemory": get(obj, "price_mem"),
            "priceSteps": get(obj, "price_step"),
        },
        "maxTxExecutionUnits": {
            "memory": as_int(obj.get("max_tx_ex_mem")),
            "steps": as_int(obj.get("max_tx_ex_steps")),
        },
        "maxBlockExecutionUnits": {
            "memory": as_int(obj.get("max_block_ex_mem")),
            "steps": as_int(obj.get("max_block_ex_steps")),
        },
        "maxValueSize": as_int(obj.get("max_val_size")),
        "collateralPercentage": get(obj, "collateral_percent"),
        "maxCollateralInputs": get(obj, "max_collateral_inputs"),
        "utxoCostPerByte": as_int(obj.get("coins_per_utxo_size")),
        "costModels": cost_models,
        // --- Conway-era governance fields ---
        "poolVotingThresholds": {
            "motionNoConfidence": get(obj, "pvt_motion_no_confidence"),
            "committeeNormal": get(obj, "pvt_committee_normal"),
            "committeeNoConfidence": get(obj, "pvt_committee_no_confidence"),
            "hardForkInitiation": get(obj, "pvt_hard_fork_initiation"),
            "ppSecurityGroup": get(obj, "pvt_p_p_security_group"),
        },
        "dRepVotingThresholds": {
            "motionNoConfidence": get(obj, "dvt_motion_no_confidence"),
            "committeeNormal": get(obj, "dvt_committee_normal"),
            "committeeNoConfidence": get(obj, "dvt_committee_no_confidence"),
            "updateToConstitution": get(obj, "dvt_update_to_constitution"),
            "hardForkInitiation": get(obj, "dvt_hard_fork_initiation"),
            "ppNetworkGroup": get(obj, "dvt_p_p_network_group"),
            "ppEconomicGroup": get(obj, "dvt_p_p_economic_group"),
            "ppTechnicalGroup": get(obj, "dvt_p_p_technical_group"),
            "ppGovGroup": get(obj, "dvt_p_p_gov_group"),
            "treasuryWithdrawal": get(obj, "dvt_treasury_withdrawal"),
        },
        "committeeMinSize": as_int(obj.get("committee_min_size")),
        "committeeMaxTermLength": as_int(obj.get("committee_max_term_length")),
        "govActionLifetime": as_int(obj.get("gov_action_lifetime")),
        "govActionDeposit": as_int(obj.get("gov_action_deposit")),
        "dRepDeposit": as_int(obj.get("drep_deposit")),
        "dRepActivity": as_int(obj.get("drep_activity")),
        "minFeeRefScriptCostPerByte": get(obj, "min_fee_ref_script_cost_per_byte"),
    });

    // Strip top-level null entries — hydra-node is strict about unknown or null
    // keys depending on the era. Fields that are absent in the Blockfrost
    // response (e.g. deprecated `minUTxOValue`) should not appear at all rather
    // than as explicit nulls.
    if let Some(map) = result.as_object_mut() {
        map.retain(|_, v| !v.is_null());
    }

    Ok(result)
}
