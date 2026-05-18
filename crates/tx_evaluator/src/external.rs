use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;

use bf_node::chain_config_watch::ChainConfigWatch;
use bf_node::pool::NodePool;
use pallas_codec::{
    minicbor::to_vec,
    utils::{AnyUInt, CborWrap},
};

use pallas_network::miniprotocols::{
    localstate::queries_v16::{
        DatumOption, PostAlonsoTransactionOutput, TransactionOutput, UTxO, Value as NetworkValue,
    },
    localtxsubmission::primitives::ScriptRef,
};
use pallas_primitives::Bytes;
use pallas_primitives::KeyValuePairs;
use pallas_traverse::MultiEraTx;
use serde::Serialize;
use tokio::sync::Mutex;

use bf_common::{
    chain_config::{ChainConfigCache, SlotConfig},
    errors::{AppError, BlockfrostError},
};
use bf_testgen::testgen::{Testgen, TestgenResponse, Variant};

use crate::{
    helper::{generate_reflection_v5, generate_reflection_v6, resolve_tx_body},
    model::api::{
        AdditionalUtxoSet, AdditionalUtxoV6, OgmiosError, OutputReferenceV6, TransactionIdV6,
    },
    native::{
        convert_to_datum_option_network, convert_to_network_value, convert_to_network_value_v6,
        create_address, extract_inputs,
    },
    ogmios5_response::invalid_request_v5,
    wrapper::{
        wrap_as_incompatible_era_v5, wrap_as_incompatible_era_v6, wrap_eval_output_v5,
        wrap_eval_output_v6, wrap_ogmios_error_v6,
    },
};

/// Evaluates transactions using the external testgen-hs Haskell binary
#[derive(Clone)]
pub struct ExternalEvaluator {
    /// Source of chain configuration
    config_watch: ChainConfigWatch,
    /// Shared mutable state holding the current testgen-hs process
    state: Arc<Mutex<EvaluatorState>>,
}

struct EvaluatorState {
    /// Running testgen-hs process, if initialized
    testgen: Option<Testgen>,
    /// The config that was used to init the current testgen process
    last_config: Option<Arc<ChainConfigCache>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct InitPayload {
    system_start: String,
    protocol_params: String,
    slot_config: SlotConfig,
    era: u16,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct EvalPayload {
    tx: String,
    utxos: String,
}

/// Either a testgen-hs response
/// or a domain-level Ogmios compatible error detected before evaluation
pub enum EvalOutput {
    Testgen(TestgenResponse),
    Error(OgmiosError),
}

/// Evaluates the given tx with utxos using the external testgen exe, which is a Haskell binary.
impl ExternalEvaluator {
    pub fn new(config_watch: ChainConfigWatch) -> Self {
        Self {
            config_watch,
            state: Arc::new(Mutex::new(EvaluatorState {
                testgen: None,
                last_config: None,
            })),
        }
    }

    /// Get or init testgen, re-spawning if config changed. Returns 503 if
    /// chain config is not yet available
    async fn ensure_testgen(&self) -> Result<Testgen, BlockfrostError> {
        let current_config = self.config_watch.get()?;

        let mut state = self.state.lock().await;

        // Config unchanged, testgen already running
        if let (Some(last), Some(testgen)) = (&state.last_config, &state.testgen)
            && Arc::ptr_eq(last, &current_config)
        {
            return Ok(testgen.clone());
        }

        tracing::info!("ExternalEvaluator: spawning testgen-hs");

        let testgen = spawn_and_init_testgen(&current_config).await.map_err(|e| {
            tracing::error!("ExternalEvaluator: failed to initialize testgen-hs: {e}");
            BlockfrostError::internal_server_error(format!(
                "Failed to initialize ExternalEvaluator: {e}"
            ))
        })?;

        state.testgen = Some(testgen.clone());
        state.last_config = Some(current_config);
        tracing::info!("ExternalEvaluator: testgen-hs initialized successfully");
        Ok(testgen)
    }

    pub async fn evaluate_binary_tx(
        &self,
        node_pool: NodePool,
        tx_cbor_binary: &[u8],
        additional_utxos: Vec<(UTxO, TransactionOutput)>,
    ) -> Result<EvalOutput, BlockfrostError> {
        let testgen = self.ensure_testgen().await?;
        let node = node_pool.get();

        let multi_era_tx = match MultiEraTx::decode(tx_cbor_binary) {
            Ok(tx) => tx,
            Err(err) => {
                let msg = format!(
                    "Invalid request: Deserialisation failure while decoding serialised transaction. CBOR failed with error: {}",
                    err
                );
                return Ok(EvalOutput::Testgen(TestgenResponse::Err(
                    serde_json::Value::String(msg),
                )));
            },
        };

        // No redeemers = no scripts to evaluate. Return empty result
        if multi_era_tx.redeemers().is_empty() {
            return Ok(EvalOutput::Testgen(TestgenResponse::Ok(serde_json::json!(
                []
            ))));
        }

        let txins = extract_inputs(&multi_era_tx)?;

        let utxos_from_node = node.await?.get_utxos_for_txins(txins).await?;

        // Merge UTxOs from node (on-chain) and user-provided additional UTxOs.
        //
        // Conflict detection (follows Ogmios's OverlappingAdditionalUtxo approach):
        //   A TxIn deterministically identifies a unique TxOut on-chain. If a user
        //   provides a TxOut for a TxIn that already exists on-chain with a *different*
        //   value, the user's data is wrong — we reject it with a 400 error. User always loses
        //   against the node and mempool data(if we had it).
        //   See: https://github.com/CardanoSolutions/ogmios/blob/bdb1bad58506e9ac470796c5e1406cde49aebc1a/server/src/Ogmios/App/Protocol/TxSubmission.hs#L628-L649
        //
        // Deduplication within additional_utxos: if the user sends the same TxIn twice,
        // the last entry wins — matching Haskell's Data.Map.fromList semantics.
        //
        // Mempool gap: we do not have mempool support as agreed before (pallas has a txmonitor
        // miniprotocol but it is not wired up in our node connection layer). Ogmios
        // merges mempool UTxOs (via LocalTxMonitor) with user additional UTxOs before
        // validating against network UTxOs. We only have network + user-provided.
        type TxInKey = (pallas_crypto::hash::Hash<32>, u64);
        let txin_key = |utxo: &UTxO| -> TxInKey { (utxo.transaction_id, u64::from(&utxo.index)) };

        let node_utxo_map: HashMap<TxInKey, (UTxO, TransactionOutput)> = utxos_from_node
            .to_vec()
            .into_iter()
            .map(|(txin, txout)| (txin_key(&txin), (txin, txout)))
            .collect();

        // Check for conflicts against on-chain UTxOs and deduplicate additional_utxos.
        let mut overlapping: Vec<OutputReferenceV6> = Vec::new();
        let mut additional_deduped: HashMap<TxInKey, (UTxO, TransactionOutput)> = HashMap::new();
        for (txin, txout) in additional_utxos {
            let key = txin_key(&txin);
            if let Some((_, node_txout)) = node_utxo_map.get(&key) {
                if node_txout != &txout {
                    overlapping.push(OutputReferenceV6 {
                        transaction: TransactionIdV6 {
                            id: key.0.to_string(),
                        },
                        index: key.1,
                    });
                }
                // Same TxIn with identical TxOut: node already has it, skip.
            } else {
                // Not on-chain. Last entry wins for duplicate TxIns within additional_utxos.
                additional_deduped.insert(key, (txin, txout));
            }
        }

        if !overlapping.is_empty() {
            return Ok(EvalOutput::Error(OgmiosError::overlapping_utxo(
                overlapping,
            )));
        }

        let utxos = KeyValuePairs::from_iter(
            node_utxo_map
                .into_values()
                .chain(additional_deduped.into_values()),
        );

        let utxos_cbor = hex::encode(to_vec(&utxos).map_err(|err| {
            BlockfrostError::internal_server_error(format!(
                "ExternalEvaluator: Failed to serialize UTxOs: {err}"
            ))
        })?);

        // TODO(testgen-hs): CBOR with empty guarded fields (e.g. empty required_signers
        // key 14 = 0x80) is rejected by testgen-hs's Conway decoder (fieldGuarded).
        // Ogmios avoids this via Babbage fallback. Adding Babbage decode to testgen-hs
        // would fix this. Until then, affected test fixtures should expect a fault response.
        let payload = EvalPayload {
            tx: hex::encode(tx_cbor_binary),
            utxos: utxos_cbor,
        };

        let json = serde_json::to_string(&payload).map_err(|err| {
            BlockfrostError::internal_server_error(format!(
                "ExternalEvaluator: Failed to serialize payload: {err}"
            ))
        })?;

        let response = testgen.send(json).await.map_err(|err| {
            BlockfrostError::internal_server_error(format!(
                "ExternalEvaluator: Failed to send payload: {err}"
            ))
        })?;
        Ok(EvalOutput::Testgen(response))
    }

    fn build_transaction_output(
        txin: UTxO,
        address: Bytes,
        script_ref: Option<CborWrap<ScriptRef>>,
        amount: NetworkValue,
        inline_datum: Option<DatumOption>,
    ) -> (UTxO, TransactionOutput) {
        let txout = TransactionOutput::Current(PostAlonsoTransactionOutput {
            address,
            script_ref,
            amount,
            inline_datum,
        });
        (txin, txout)
    }

    /// Decodes raw payload (hex/base64/binary) then evaluates. Used by application/cbor endpoint.
    pub async fn evaluate_tx_payload_v5(
        &self,
        node_pool: NodePool,
        tx_body: &[u8],
        additional_utxos: Option<AdditionalUtxoSet>,
    ) -> Result<serde_json::Value, BlockfrostError> {
        let tx_cbor_binary = match resolve_tx_body(tx_body) {
            Ok(decoded) => decoded,
            Err(_) => return Ok(invalid_request_v5(&generate_reflection_v5())),
        };
        self.evaluate_binary_tx_v5(node_pool, &tx_cbor_binary, additional_utxos)
            .await
    }

    /// Decodes raw payload (hex/base64/binary) then evaluates. Used by application/cbor endpoint.
    pub async fn evaluate_tx_payload_v6(
        &self,
        node_pool: NodePool,
        tx_body: &[u8],
        additional_utxos: Option<Vec<AdditionalUtxoV6>>,
    ) -> Result<serde_json::Value, BlockfrostError> {
        let tx_cbor_binary = match resolve_tx_body(tx_body) {
            Ok(decoded) => decoded,
            Err(msg) => {
                let oe = OgmiosError::invalid_request(msg);
                return Ok(wrap_ogmios_error_v6(&oe, &generate_reflection_v6()));
            },
        };
        self.evaluate_binary_tx_v6(node_pool, &tx_cbor_binary, additional_utxos)
            .await
    }

    /// Evaluates already-decoded CBOR binary. Used by JSON endpoints (utxos.rs).
    pub async fn evaluate_binary_tx_v5(
        &self,
        node_pool: NodePool,
        tx_cbor_binary: &[u8],
        additional_utxos: Option<AdditionalUtxoSet>,
    ) -> Result<serde_json::Value, BlockfrostError> {
        // Pre-Alonzo tx (3-element CBOR array) cannot be evaluated.
        if tx_cbor_binary.first() == Some(&0x83) {
            return Ok(wrap_as_incompatible_era_v5("Mary".to_string()));
        }

        let user_utxos = additional_utxos
            .unwrap_or_default()
            .iter()
            .map(
                |(utxo, tout)| -> Result<(UTxO, TransactionOutput), BlockfrostError> {
                    let txin = UTxO {
                        transaction_id: pallas_crypto::hash::Hash::<32>::from_str(&utxo.tx_id)
                            .map_err(|e| {
                                BlockfrostError::custom_400(format!(
                                    "invalid tx_id '{}': {e}",
                                    utxo.tx_id
                                ))
                            })?,
                        index: AnyUInt::U64(utxo.index),
                    };
                    Ok(Self::build_transaction_output(
                        txin,
                        create_address(&tout.address)?,
                        tout.script
                            .as_ref()
                            .map(|s| ScriptRef::try_from(s.clone()).map(CborWrap))
                            .transpose()?,
                        convert_to_network_value(&tout.value)?,
                        convert_to_datum_option_network(&tout.datum, &tout.datum_hash)?,
                    ))
                },
            )
            .collect::<Result<Vec<_>, _>>()?;

        Ok(wrap_eval_output_v5(
            self.evaluate_binary_tx(node_pool, tx_cbor_binary, user_utxos)
                .await?,
        ))
    }

    /// Evaluates already-decoded CBOR binary. Used by JSON endpoints (utxos.rs).
    pub async fn evaluate_binary_tx_v6(
        &self,
        node_pool: NodePool,
        tx_cbor_binary: &[u8],
        additional_utxos: Option<Vec<AdditionalUtxoV6>>,
    ) -> Result<serde_json::Value, BlockfrostError> {
        // Pre-Alonzo tx (3-element CBOR array) cannot be evaluated.
        if tx_cbor_binary.first() == Some(&0x83) {
            return Ok(wrap_as_incompatible_era_v6("mary".to_string()));
        }

        let user_utxos = additional_utxos
            .unwrap_or_default()
            .iter()
            .map(
                |utxo| -> Result<(UTxO, TransactionOutput), BlockfrostError> {
                    let txin = UTxO {
                        transaction_id: pallas_crypto::hash::Hash::<32>::from_str(
                            utxo.transaction_id(),
                        )
                        .map_err(|e| {
                            BlockfrostError::custom_400(format!(
                                "invalid transaction id '{}': {e}",
                                utxo.transaction_id()
                            ))
                        })?,
                        index: AnyUInt::U64(utxo.output_index()),
                    };
                    Ok(Self::build_transaction_output(
                        txin,
                        create_address(&utxo.address)?,
                        utxo.script
                            .as_ref()
                            .map(|s| ScriptRef::try_from(s).map(CborWrap))
                            .transpose()?,
                        convert_to_network_value_v6(&utxo.value)?,
                        convert_to_datum_option_network(&utxo.datum, &utxo.datum_hash)?,
                    ))
                },
            )
            .collect::<Result<Vec<_>, _>>()?;

        Ok(wrap_eval_output_v6(
            self.evaluate_binary_tx(node_pool, tx_cbor_binary, user_utxos)
                .await?,
        ))
    }
}

/// Spawn a fresh testgen-hs process and send the init payload
/// TODO(testgen-hs): Currently testgen only accepts init message as the first message.
/// Support receiving init message anytime so we won't need to kill&spawn
async fn spawn_and_init_testgen(config: &ChainConfigCache) -> Result<Testgen, AppError> {
    let system_start = to_vec(&config.genesis_config.system_start).map_err(|err| {
        AppError::Server(format!(
            "ExternalEvaluator: failed to serialize genesis config: {err}"
        ))
    })?;

    let protocol_params = to_vec(&config.protocol_params).map_err(|err| {
        AppError::Server(format!(
            "ExternalEvaluator: failed to serialize protocol params: {err}"
        ))
    })?;

    let init_payload = InitPayload {
        system_start: hex::encode(system_start),
        protocol_params: hex::encode(protocol_params),
        slot_config: config.slot_config.clone(),
        era: config.era,
    };

    let payload = serde_json::to_string(&init_payload).map_err(|err| {
        AppError::Server(format!(
            "ExternalEvaluator: failed to serialize initial payload: {err}"
        ))
    })?;

    let testgen = Testgen::spawn_with_init(Variant::EvaluateStream, payload)
        .map_err(|err| AppError::Server(format!("Failed to spawn ExternalEvaluator: {err}")))?;

    Ok(testgen)
}
