use anyhow::{Result, anyhow, bail};
use blockfrost::Pagination;
use cardano_serialization_lib::{
    Address, BigNum, FixedTransaction, LinearFee, TransactionBuilder, TransactionBuilderConfig,
    TransactionBuilderConfigBuilder, TransactionHash, TransactionInput, TransactionOutput,
    TransactionUnspentOutput, TransactionUnspentOutputs,
};
use serde_json::Value;
use std::path::Path;
use tracing::info;

use bf_common::cardano_keys;

const MIN_OUTPUT_LOVELACE: u64 = 840_450;

/// CBOR prefix for a 32-byte bytestring (`5820` in hex).
const CBOR_32_PREFIX: &str = "5820";

/// FIXME: proper errors, not `anyhow!`
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
    /// given `payment.skey`.
    pub(super) async fn lovelace_on_payment_skey(&self, skey_path: &Path) -> Result<u64> {
        let address = self.derive_enterprise_address_from_skey(skey_path)?;
        self.lovelace_on_addr(&address).await
    }

    /// Check how much lovelace is available on an address (via Blockfrost).
    pub(super) async fn lovelace_on_addr(&self, address: &str) -> Result<u64> {
        let utxos = self
            .blockfrost_api()?
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

    /// Derive an enterprise (payment-only) bech32 address from a signing-key (sync).
    pub(super) fn derive_enterprise_address_from_skey(&self, skey_path: &Path) -> Result<String> {
        cardano_keys::derive_enterprise_address(skey_path, self.config.network.as_str())
    }

    /// Derive an enterprise address from a vkey JSON envelope (sync).
    ///
    /// The vkey envelope has the form:
    /// ```json
    /// { "type": "PaymentVerificationKeyShelley_ed25519",
    ///   "description": "...",
    ///   "cborHex": "5820<64-hex-chars>" }
    /// ```
    /// We extract the raw public key bytes from the CBOR-encoded `cborHex`
    /// field and call `cardano_keys::enterprise_address_from_pubkey()`.
    pub(super) fn derive_enterprise_address_from_vkey_json(
        &self,
        vkey_json: &serde_json::Value,
    ) -> Result<String> {
        let cbor_hex = vkey_json
            .get("cborHex")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("vkey envelope missing cborHex field"))?;

        let raw_hex = cbor_hex
            .strip_prefix(CBOR_32_PREFIX)
            .ok_or_else(|| anyhow!("vkey cborHex does not start with {CBOR_32_PREFIX}"))?;

        let pub_key_bytes = hex::decode(raw_hex)?;
        let pub_key = cardano_serialization_lib::PublicKey::from_bytes(&pub_key_bytes)
            .map_err(|e| anyhow!("invalid public key bytes: {e}"))?;

        cardano_keys::enterprise_address_from_pubkey(&pub_key, self.config.network.as_str())
    }

    /// Generate a new Cardano keypair in cardano-cli envelope format (sync).
    pub(super) fn new_cardano_keypair(base_path: &Path) -> Result<()> {
        cardano_keys::generate_keypair(base_path)
    }

    /// Query UTxOs for an address via Blockfrost and return them in
    /// cardano-cli `query utxo --output-json` format, which is what
    /// hydra-node's `/commit` endpoint expects.
    pub(super) async fn query_utxo_json(&self, address: &str) -> Result<serde_json::Value> {
        let utxos = self
            .blockfrost_api()?
            .addresses_utxos(address, Pagination::all())
            .await;

        let utxos = match utxos {
            Ok(u) => u,
            Err(e) => {
                let msg = e.to_string();
                if msg.contains("404") {
                    return Ok(serde_json::json!({}));
                }
                bail!("blockfrost addresses_utxos failed: {e}");
            },
        };

        // Build a JSON object: { "txhash#idx": { "address": ..., "value": { "lovelace": N } } }
        let mut obj = serde_json::Map::new();
        for utxo in &utxos {
            let key = format!("{}#{}", utxo.tx_hash, utxo.output_index);
            let lovelace = utxo
                .amount
                .iter()
                .find(|a| a.unit == "lovelace")
                .map(|a| a.quantity.clone())
                .unwrap_or_else(|| "0".to_string());

            let mut value_map = serde_json::Map::new();
            let lovelace_u64: u64 = lovelace
                .parse()
                .map_err(|e| anyhow!("bad lovelace quantity {lovelace:?} on {key}: {e}"))?;
            value_map.insert(
                "lovelace".to_string(),
                serde_json::Value::Number(lovelace_u64.into()),
            );

            // Include native assets if any
            for asset in &utxo.amount {
                if asset.unit != "lovelace" {
                    let policy_id = &asset.unit[..56];
                    let asset_name = &asset.unit[56..];
                    let policy_entry = value_map
                        .entry(policy_id.to_string())
                        .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
                    if let Some(policy_obj) = policy_entry.as_object_mut() {
                        let asset_qty: u64 = asset.quantity.parse().map_err(|e| {
                            anyhow!(
                                "bad asset quantity {:?} for {}/{} on {}: {}",
                                asset.quantity,
                                policy_id,
                                asset_name,
                                key,
                                e,
                            )
                        })?;
                        policy_obj.insert(
                            asset_name.to_string(),
                            serde_json::Value::Number(asset_qty.into()),
                        );
                    }
                }
            }

            let entry = serde_json::json!({
                "address": address,
                "datum": null,
                "datumhash": null,
                "inlineDatum": null,
                "referenceScript": null,
                "value": value_map,
            });
            obj.insert(key, entry);
        }

        Ok(serde_json::Value::Object(obj))
    }

    /// Build, sign, and submit an L1 transaction that sends `amount_lovelace`
    /// from `addr_from` to `addr_to`, using CSL + Blockfrost.
    pub(super) async fn fund_address(
        &self,
        addr_from: &str,
        addr_to: &str,
        amount_lovelace: u64,
        payment_skey_path: &Path,
    ) -> Result<()> {
        use cardano_serialization_lib::CoinSelectionStrategyCIP2;

        let priv_key = cardano_keys::load_private_key(payment_skey_path)?;
        let bf = self.blockfrost_api()?;

        // Fetch UTxOs via Blockfrost (limit to 200 to avoid MaxTxSizeUTxO)
        let utxos = bf.addresses_utxos(addr_from, Pagination::all()).await?;

        if utxos.is_empty() {
            bail!("no UTxOs found for addr_from");
        }

        // Fetch protocol parameters for fee calculation
        let params = bf.epochs_latest_parameters().await?;
        let params_json = serde_json::to_value(&params)?;
        let builder_config = tx_builder_config_from_params(&params_json)?;

        // Fetch current slot for TTL
        let latest_block = bf.blocks_latest().await?;
        let current_slot = latest_block
            .slot
            .ok_or_else(|| anyhow!("latest block missing slot"))? as u64;

        let mut tx_builder = TransactionBuilder::new(&builder_config);

        let output_addr = Address::from_bech32(addr_to)?;
        let change_addr = Address::from_bech32(addr_from)?;

        let ttl = current_slot + 7200;
        tx_builder.set_ttl_bignum(&BigNum::from_str(&ttl.to_string())?);

        let output_value =
            cardano_serialization_lib::Value::new(&BigNum::from_str(&amount_lovelace.to_string())?);
        tx_builder.add_output(&TransactionOutput::new(&output_addr, &output_value))?;

        // Build unspent outputs (lovelace-only, max 200)
        let mut unspent_outputs = TransactionUnspentOutputs::new();
        for utxo in utxos.iter().take(200) {
            if utxo.amount.iter().all(|a| a.unit == "lovelace") {
                if let Some(token) = utxo.amount.iter().find(|a| a.unit == "lovelace") {
                    let input_value =
                        cardano_serialization_lib::Value::new(&BigNum::from_str(&token.quantity)?);
                    let tx_hash_bytes = hex::decode(&utxo.tx_hash)?;
                    let tx_hash = TransactionHash::from_bytes(tx_hash_bytes)?;
                    let input = TransactionInput::new(&tx_hash, utxo.output_index.try_into()?);
                    let output = TransactionOutput::new(&change_addr, &input_value);
                    unspent_outputs.add(&TransactionUnspentOutput::new(&input, &output));
                }
            }
        }

        tx_builder.add_inputs_from(&unspent_outputs, CoinSelectionStrategyCIP2::LargestFirst)?;
        tx_builder.add_change_if_needed(&change_addr)?;

        let tx_body = tx_builder.build()?;
        let mut fixed_tx = FixedTransaction::new_from_body_bytes(&tx_body.to_bytes())?;
        fixed_tx.sign_and_add_vkey_signature(&priv_key)?;

        // Submit via Blockfrost
        let tx_cbor = fixed_tx.to_bytes();
        self.blockfrost_api()?
            .transactions_submit(tx_cbor.to_vec())
            .await?;

        Ok(())
    }

    /// Commit all UTxOs from `from_addr` into a Hydra Head via the
    /// hydra-node `/commit` endpoint. Signs and submits the resulting L1
    /// transaction.
    pub(super) async fn commit_all_utxo_to_hydra(
        &self,
        from_addr: &str,
        hydra_api_port: u16,
        commit_funds_skey: &Path,
    ) -> Result<()> {
        use anyhow::Context;
        use reqwest::header;

        // Query UTxOs via Blockfrost, convert to cardano-cli JSON shape that
        // hydra-node /commit expects.
        let utxo_json = self.query_utxo_json(from_addr).await?;
        let utxo_body = serde_json::to_vec(&utxo_json).context("failed to serialize utxo JSON")?;

        // POST to hydra-node /commit
        let url = format!("http://127.0.0.1:{hydra_api_port}/commit");
        let resp = self
            .http
            .post(url)
            .header(header::CONTENT_TYPE, "application/json")
            .body(utxo_body)
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

        let signed_tx = cardano_keys::sign_tx_envelope(&tx_envelope, commit_funds_skey)?;
        let signed_cbor_hex = signed_tx["cborHex"]
            .as_str()
            .ok_or_else(|| anyhow!("signed tx missing cborHex"))?;
        let signed_cbor = hex::decode(signed_cbor_hex)?;

        // Submit via Blockfrost.
        self.blockfrost_api()?
            .transactions_submit(signed_cbor.to_vec())
            .await?;

        Ok(())
    }

    /// Build, sign, and send an L2 (Hydra) transaction (fee=0) via WebSocket.
    pub(super) async fn send_hydra_transaction(
        &self,
        hydra_api_port: u16,
        sender_addr: &str,
        receiver_addr: &str,
        sender_skey_path: &Path,
        amount_lovelace: u64,
    ) -> Result<()> {
        use anyhow::Context;

        fn utxo_lovelace(entry: &Value) -> Option<u64> {
            if let Some(v) = entry.pointer("/value/lovelace") {
                if let Some(n) = v.as_u64() {
                    return Some(n);
                }
                if let Some(s) = v.as_str() {
                    return s.parse().ok();
                }
            }

            if let Some(amounts) = entry.get("amount").and_then(Value::as_array) {
                for item in amounts {
                    if item.get("unit").and_then(Value::as_str) == Some("lovelace") {
                        if let Some(q) = item.get("quantity") {
                            if let Some(n) = q.as_u64() {
                                return Some(n);
                            }
                            if let Some(s) = q.as_str() {
                                return s.parse().ok();
                            }
                        }
                    }
                }
            }

            None
        }

        let snapshot_url = format!("http://127.0.0.1:{hydra_api_port}/snapshot/utxo");
        let utxo: Value = self
            .http
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
            if v.get("address").and_then(Value::as_str) == Some(sender_addr) {
                filtered.insert(k.clone(), v.clone());
            }
        }

        if amount_lovelace < MIN_OUTPUT_LOVELACE {
            bail!(
                "amount_lovelace {amount_lovelace} is below minimum output lovelace {MIN_OUTPUT_LOVELACE}"
            );
        }

        let mut candidates: Vec<(String, u64)> = Vec::new();
        for (k, v) in filtered.iter() {
            let lovelace_total = utxo_lovelace(v).context("utxo entry: expected lovelace value")?;
            candidates.push((k.clone(), lovelace_total));
        }

        if candidates.is_empty() {
            bail!("no UTxO found for sender address");
        }

        candidates.sort_by_key(|(_, value)| *value);

        let min_total = amount_lovelace
            .checked_add(MIN_OUTPUT_LOVELACE)
            .ok_or_else(|| anyhow!("amount_lovelace overflow"))?;

        let mut selected: Vec<String> = Vec::new();
        let mut selected_total: u64 = 0;

        if let Some((tx_in, total)) = candidates
            .iter()
            .find(|(_, total)| *total == amount_lovelace)
        {
            selected.push(tx_in.clone());
            selected_total = *total;
        } else if let Some((tx_in, total)) = candidates
            .iter()
            .find(|(_, total)| *total > amount_lovelace && *total >= min_total)
        {
            selected.push(tx_in.clone());
            selected_total = *total;
        } else {
            for (tx_in, total) in &candidates {
                selected_total = selected_total
                    .checked_add(*total)
                    .ok_or_else(|| anyhow!("utxo sum overflow"))?;
                selected.push(tx_in.clone());
                if selected_total == amount_lovelace || selected_total >= min_total {
                    break;
                }
            }
        }

        if selected_total < amount_lovelace {
            bail!("insufficient lovelace in available UTxOs: {selected_total} < {amount_lovelace}");
        }

        let change = selected_total - amount_lovelace;
        if change > 0 && change < MIN_OUTPUT_LOVELACE {
            bail!("change output {change} is below minimum output lovelace {MIN_OUTPUT_LOVELACE}");
        }

        // Build the L2 transaction body using CSL (fee = 0).
        let priv_key = cardano_keys::load_private_key(sender_skey_path)?;

        let receiver = Address::from_bech32(receiver_addr)?;
        let sender = Address::from_bech32(sender_addr)?;

        let mut inputs = Vec::new();
        let mut outputs = Vec::new();

        // Parse selected UTxOs into CSL inputs
        for tx_in_str in &selected {
            let parts: Vec<&str> = tx_in_str.split('#').collect();
            if parts.len() != 2 {
                bail!("invalid tx_in format: {tx_in_str}");
            }
            let hash_bytes = hex::decode(parts[0])?;
            let tx_hash = TransactionHash::from_bytes(hash_bytes)?;
            let index: u32 = parts[1].parse()?;
            inputs.push(TransactionInput::new(&tx_hash, index));
        }

        // Receiver output
        outputs.push(TransactionOutput::new(
            &receiver,
            &cardano_serialization_lib::Value::new(&BigNum::from_str(
                &amount_lovelace.to_string(),
            )?),
        ));

        // Change output
        if change > 0 {
            outputs.push(TransactionOutput::new(
                &sender,
                &cardano_serialization_lib::Value::new(&BigNum::from_str(&change.to_string())?),
            ));
        }

        // Build raw transaction body (fee=0 for L2)
        let tx_body = cardano_serialization_lib::TransactionBody::new_tx_body(
            &{
                let mut ins = cardano_serialization_lib::TransactionInputs::new();
                for i in &inputs {
                    ins.add(i);
                }
                ins
            },
            &{
                let mut outs = cardano_serialization_lib::TransactionOutputs::new();
                for o in &outputs {
                    outs.add(o);
                }
                outs
            },
            &BigNum::zero(), // fee = 0
        );

        let mut fixed_tx = FixedTransaction::new_from_body_bytes(&tx_body.to_bytes())?;
        fixed_tx.sign_and_add_vkey_signature(&priv_key)?;

        // Wrap in cardano-cli envelope format for Hydra
        let tx_signed = serde_json::json!({
            "type": "Witnessed Tx ConwayEra",
            "description": "",
            "cborHex": hex::encode(fixed_tx.to_bytes()),
        });

        let payload = serde_json::json!({
            "tag": "NewTx",
            "transaction": tx_signed,
        });

        tracing::info!(
            "sending WebSocket payload: {}",
            serde_json::to_string(&payload)?
        );

        let ws_url = format!("ws://127.0.0.1:{hydra_api_port}/");
        send_one_websocket_msg(&ws_url, payload, std::time::Duration::from_secs(2)).await?;

        Ok(())
    }
}

// FIXME: The `blockfrost` SDK crate (v1.2.1) does not re-export `EpochParamContent`
// from its public API, so we work with `serde_json::Value` here.
fn tx_builder_config_from_params(params: &serde_json::Value) -> Result<TransactionBuilderConfig> {
    let get_str = |key: &str| -> Result<String> {
        params
            .get(key)
            .ok_or_else(|| anyhow!("missing field {key}"))
            .and_then(|v| match v {
                serde_json::Value::String(s) => Ok(s.clone()),
                serde_json::Value::Number(n) => Ok(n.to_string()),
                _ => bail!("unexpected type for {key}"),
            })
    };

    let config = TransactionBuilderConfigBuilder::new()
        .fee_algo(&LinearFee::new(
            &BigNum::from_str(&get_str("min_fee_a")?)?,
            &BigNum::from_str(&get_str("min_fee_b")?)?,
        ))
        .pool_deposit(&BigNum::from_str(&get_str("pool_deposit")?)?)
        .key_deposit(&BigNum::from_str(&get_str("key_deposit")?)?)
        .coins_per_utxo_byte(&BigNum::from_str(&get_str("coins_per_utxo_size")?)?)
        .max_value_size(get_str("max_val_size")?.parse::<u32>()?)
        .max_tx_size(get_str("max_tx_size")?.parse::<u32>()?)
        .build()?;
    Ok(config)
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

#[cfg(unix)]
pub fn sigterm(pid: u32) -> Result<()> {
    use nix::sys::signal::{Signal, kill};
    use nix::unistd::Pid;
    Ok(kill(Pid::from_raw(pid as i32), Signal::SIGTERM)?)
}

#[cfg(windows)]
pub fn sigterm(_pid: u32) -> Result<()> {
    unimplemented!("SIGTERM is not available on Windows, but there's no `hydra-node` to run there")
}

/// We use it for `localhost` tests, to detect if the Gateway and Bridge are
/// running on the same host. Then we cannot set up a
/// `[bf_common::tcp_mux_tunnel::Tunnel]`, because the ports are already taken.
pub fn hashed_machine_id() -> String {
    const MACHINE_ID_NAMESPACE: &str = "blockfrost.machine-id.v1";

    let mut hasher = blake3::Hasher::new();
    hasher.update(MACHINE_ID_NAMESPACE.as_bytes());
    hasher.update(b":");

    match machine_uid::get() {
        Ok(id) => {
            hasher.update(id.as_bytes());
        },
        Err(e) => {
            tracing::warn!(error = ?e, "machine_uid::get() failed; falling back to random bytes");
            let mut fallback = [0u8; 32];
            getrandom::fill(&mut fallback)
                .expect("getrandom::fill shouldn’t fail in normal circumstances");
            hasher.update(&fallback);
        },
    }

    hasher.finalize().to_hex().to_string()
}
