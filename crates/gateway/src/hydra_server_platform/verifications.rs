use anyhow::{Result, anyhow};
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

/// FIXME: proper errors, not `anyhow!`
impl super::HydraConfig {
    /// Generates Hydra keys if they don’t exist.
    pub(super) async fn gen_hydra_keys(&self, target_dir: &Path) -> Result<()> {
        std::fs::create_dir_all(target_dir)?;

        let key_path = target_dir.join("hydra.sk");

        if !key_path.exists() {
            info!("hydra-controller: generating hydra keys");

            let status = tokio::process::Command::new(&self.hydra_node_exe)
                .arg("gen-hydra-key")
                .arg("--output-file")
                .arg(target_dir.join("hydra"))
                .status()
                .await?;

            if !status.success() {
                Err(anyhow!("gen-hydra-key failed with status: {status}"))?;
            }
        } else {
            info!("hydra-controller: hydra keys already exist");
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
                    Err(anyhow!("blockfrost addresses_utxos failed: {e}"))
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

    /// Generate a new Cardano keypair in cardano-cli envelope format.
    pub fn new_cardano_keypair(base_path: &Path) -> Result<()> {
        cardano_keys::generate_keypair(base_path)
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

        // Fetch UTxOs via Blockfrost (limit to 200 to avoid MaxTxSizeUTxO)
        let utxos = self
            .blockfrost_api
            .addresses_utxos(addr_from, Pagination::all())
            .await?;

        if utxos.is_empty() {
            return Err(anyhow!("no UTxOs found for addr_from"));
        }

        // Fetch protocol parameters for fee calculation
        let params = self.blockfrost_api.epochs_latest_parameters().await?;
        let params_json = serde_json::to_value(&params)?;
        let builder_config = tx_builder_config_from_params(&params_json)?;

        // Fetch current slot for TTL
        let latest_block = self.blockfrost_api.blocks_latest().await?;
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
        self.blockfrost_api
            .transactions_submit(tx_cbor.to_vec())
            .await?;

        Ok(())
    }

    /// Commit all UTxOs from `from_addr` into a Hydra Head via the
    /// hydra-node `/commit` endpoint. Signs and submits the resulting L1
    /// transaction.
    ///
    /// The commit wallet must be funded *before* Init so that the signing
    /// key's UTxO set is not disturbed between Init and Commit (the
    /// hydra-node uses signing key UTxOs for collateral).
    pub(super) async fn commit_all_utxo_to_hydra(
        &self,
        from_addr: &str,
        hydra_api_port: u16,
        commit_funds_skey: &Path,
    ) -> Result<()> {
        use anyhow::Context;
        use reqwest::header;

        // 1. Query UTxOs via Blockfrost, convert to cardano-cli JSON shape
        //    that hydra-node /commit expects.
        let utxo_json = self.query_utxo_json(from_addr).await?;
        let utxo_body = serde_json::to_vec(&utxo_json).context("failed to serialize utxo JSON")?;

        // 2. POST to hydra-node /commit
        let url = format!("http://127.0.0.1:{hydra_api_port}/commit");
        let client = reqwest::Client::new();
        let resp = client
            .post(url)
            .header(header::CONTENT_TYPE, "application/json")
            .body(utxo_body)
            .send()
            .await
            .context("failed to POST /commit to hydra-node")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.bytes().await.unwrap_or_default();
            return Err(anyhow!(
                "hydra /commit failed with {}: {}",
                status,
                String::from_utf8_lossy(&body)
            ));
        }

        let commit_tx_bytes = resp
            .bytes()
            .await
            .context("failed to read hydra /commit response body")?
            .to_vec();

        // 3. Sign with CSL
        let tx_envelope: serde_json::Value = serde_json::from_slice(&commit_tx_bytes)
            .context("hydra /commit response was not valid JSON")?;

        let signed_tx = cardano_keys::sign_tx_envelope(&tx_envelope, commit_funds_skey)?;
        let signed_cbor_hex = signed_tx["cborHex"]
            .as_str()
            .ok_or_else(|| anyhow!("signed tx missing cborHex"))?;
        let signed_cbor = hex::decode(signed_cbor_hex)?;

        // 4. Submit via Blockfrost.
        self.blockfrost_api
            .transactions_submit(signed_cbor.to_vec())
            .await?;

        Ok(())
    }

    /// Query UTxOs for an address via Blockfrost and return them in
    /// cardano-cli `query utxo --output-json` format, which is what
    /// hydra-node's `/commit` endpoint expects.
    pub(super) async fn query_utxo_json(&self, address: &str) -> Result<serde_json::Value> {
        let utxos = self
            .blockfrost_api
            .addresses_utxos(address, Pagination::all())
            .await;

        let utxos = match utxos {
            Ok(u) => u,
            Err(e) => {
                let msg = e.to_string();
                if msg.contains("404") {
                    return Ok(serde_json::json!({}));
                }
                return Err(anyhow!("blockfrost addresses_utxos failed: {e}"));
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
            value_map.insert(
                "lovelace".to_string(),
                serde_json::Value::Number(lovelace.parse::<u64>().unwrap_or(0).into()),
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
                        policy_obj.insert(
                            asset_name.to_string(),
                            serde_json::Value::Number(
                                asset.quantity.parse::<u64>().unwrap_or(0).into(),
                            ),
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

        const MIN_OUTPUT_LOVELACE: u64 = super::MIN_LOVELACE_PER_TRANSACTION;

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
            if v.get("address").and_then(Value::as_str) == Some(sender_addr) {
                filtered.insert(k.clone(), v.clone());
            }
        }

        if amount_lovelace < MIN_OUTPUT_LOVELACE {
            return Err(anyhow!(
                "amount_lovelace {amount_lovelace} is below minimum output lovelace {MIN_OUTPUT_LOVELACE}"
            ));
        }

        // Collect candidates sorted by lovelace value (ascending).
        let mut candidates: Vec<(String, u64)> = Vec::new();
        for (k, v) in filtered.iter() {
            let lovelace_total = utxo_lovelace(v).context("utxo entry: expected lovelace value")?;
            candidates.push((k.clone(), lovelace_total));
        }

        if candidates.is_empty() {
            return Err(anyhow!("no UTxO found for sender address"));
        }

        candidates.sort_by_key(|(_, value)| *value);

        let min_total = amount_lovelace
            .checked_add(MIN_OUTPUT_LOVELACE)
            .ok_or_else(|| anyhow!("amount_lovelace overflow"))?;

        // Selection strategy:
        //  1. Exact match - no change output needed.
        //  2. Single UTxO large enough for amount + min-change.
        //  3. Aggregate smallest-first until we have enough.
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
            return Err(anyhow!(
                "insufficient lovelace in available UTxOs: {selected_total} < {amount_lovelace}"
            ));
        }

        let change = selected_total - amount_lovelace;
        if change > 0 && change < MIN_OUTPUT_LOVELACE {
            return Err(anyhow!(
                "change output {change} is below minimum output lovelace {MIN_OUTPUT_LOVELACE}"
            ));
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
                return Err(anyhow!("invalid tx_in format: {tx_in_str}"));
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
        let mut tx_body_builder = cardano_serialization_lib::TransactionBody::new_tx_body(
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
        // TTL is not needed for L2 transactions, but set a high one just in case
        tx_body_builder.set_ttl(&BigNum::from_str("99999999999")?);

        let mut fixed_tx = FixedTransaction::new_from_body_bytes(&tx_body_builder.to_bytes())?;
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
            "hydra-controller: sending WebSocket payload: {}",
            serde_json::to_string(&payload)?
        );

        let ws_url = format!("ws://127.0.0.1:{hydra_api_port}/");
        send_one_websocket_msg(&ws_url, payload, std::time::Duration::from_secs(2)).await?;

        Ok(())
    }

    pub(super) async fn hydra_utxo_count(&self, hydra_api_port: u16) -> Result<u64> {
        use anyhow::Context;

        let url = format!("http://127.0.0.1:{hydra_api_port}/snapshot/utxo");

        let v: Value = reqwest::Client::new()
            .get(&url)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await
            .context("snapshot/utxo: failed to decode JSON")?;

        v.as_object()
            .context("snapshot/utxo: expected top-level JSON object")?
            .len()
            .try_into()
            .context("utxo length does not fit into u64 (?)")
    }
}

// FIXME: The `blockfrost` SDK crate (v1.2.1) does not re-export `EpochParamContent`
// from its public API, so we work with `serde_json::Value` here. Once the crate
// properly exports the type, this function should accept `&blockfrost::EpochParamContent`
// directly instead.
fn tx_builder_config_from_params(params: &serde_json::Value) -> Result<TransactionBuilderConfig> {
    let get_str = |key: &str| -> Result<String> {
        params
            .get(key)
            .ok_or_else(|| anyhow!("missing field {key}"))
            .and_then(|v| match v {
                serde_json::Value::String(s) => Ok(s.clone()),
                serde_json::Value::Number(n) => Ok(n.to_string()),
                _ => Err(anyhow!("unexpected type for {key}")),
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
        return Err(anyhow!("metric {metric} not found in /metrics output"));
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

    // Drain until we observe the close handshake (or the peer drops):
    while let Some(msg) = read.next().await {
        match msg? {
            Message::Close(_) => break,
            Message::Text(msg) => {
                tracing::info!("hydra-controller: got WebSocket message: {}", msg)
            },
            msg => tracing::info!("hydra-controller: got WebSocket message: {:?}", msg),
        }
    }

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
/// Fanout transaction can be theoretically submitted… and still sometimes fail…
/// (we retry then).
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

#[cfg(unix)]
pub fn sigterm(pid: u32) -> Result<()> {
    use nix::sys::signal::{Signal, kill};
    use nix::unistd::Pid;
    Ok(kill(Pid::from_raw(pid as i32), Signal::SIGTERM)?)
}

#[cfg(windows)]
pub fn sigterm(_pid: u32) -> Result<()> {
    unreachable!()
}

/// We use it for `localhost` tests, to detect if the Gateway and Platform are
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
                .expect("getrandom::fill shouldn't fail in normal circumstances");
            hasher.update(&fallback);
        },
    }

    hasher.finalize().to_hex().to_string()
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
