use std::str::FromStr;
use std::sync::Arc;

use bf_common::{
    chain_config::{ChainConfigCache, SlotConfig},
    errors::{AppError, BlockfrostError},
};
use bf_node::chain_config_watch::ChainConfigWatch;
use bf_node::pool::NodePool;
use bf_testgen::testgen::{Testgen, TestgenResponse};
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

use crate::{
    model::api::{AdditionalUtxoSet, AdditionalUtxoV6},
    native::{
        convert_to_datum_option_network, convert_to_network_value, convert_to_network_value_v6,
        create_address, extract_inputs,
    },
    wrapper::{wrap_response_v5, wrap_response_v6},
};

/// Evaluates transactions using the external testgen-hs Haskell binary.
///
/// The evaluator is pull-based: on each request it reads the current
/// [`ChainConfigCache`] from [`ChainConfigWatch`]. If the config is not yet
/// available (node still syncing), it returns 503. If the config has changed
/// since the last init (e.g. protocol parameter update at an epoch boundary),
/// it re-spawns testgen-hs with the new parameters.
///
/// testgen-hs does not support reinit, so a process restart is required on
/// config change.
#[derive(Clone)]
pub struct ExternalEvaluator {
    /// Source of chain configuration (lazy, refreshed at epoch boundaries).
    config_watch: ChainConfigWatch,
    /// Shared mutable state holding the current testgen-hs process.
    state: Arc<Mutex<EvaluatorState>>,
}

struct EvaluatorState {
    /// Running testgen-hs process, if initialized.
    testgen: Option<Testgen>,
    /// The config that was used to init the current testgen process.
    /// `Arc::ptr_eq` is used to detect config changes from `ChainConfigWatch`.
    last_config: Option<Arc<ChainConfigCache>>,
}

/// JSON payload sent to testgen-hs on first line to initialize the evaluator.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct InitPayload {
    /// CBOR-encoded Shelley genesis `system_start`, hex string.
    system_start: String,
    /// CBOR-encoded current protocol parameters, hex string.
    protocol_params: String,
    /// Slot timing configuration for epoch/slot calculations.
    slot_config: SlotConfig,
    /// Cardano era index (see [`ChainConfigCache::CONWAY_ERA`]).
    era: u16,
}

/// JSON payload sent to testgen-hs for each evaluation request.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct EvalPayload {
    /// CBOR-encoded transaction, hex string.
    tx: String,
    /// CBOR-encoded UTxO set (node UTxOs merged with user-provided additional UTxOs), hex string.
    utxos: String,
}

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
    /// chain config is not yet available.
    async fn ensure_testgen(&self) -> Result<Testgen, BlockfrostError> {
        let current_config = self.config_watch.get()?;

        // Fast path: config unchanged, testgen already running.
        {
            let state = self.state.lock().await;
            if let (Some(ref last), Some(ref testgen)) = (&state.last_config, &state.testgen) {
                if Arc::ptr_eq(last, &current_config) {
                    return Ok(testgen.clone());
                }
            }
        }
        // Lock released — spawn outside the lock to avoid head-of-line blocking.
        tracing::info!("ExternalEvaluator: spawning testgen-hs");

        let testgen = spawn_and_init_testgen(&current_config).await.map_err(|e| {
            tracing::error!("ExternalEvaluator: failed to initialize testgen-hs: {e}");
            BlockfrostError::internal_server_error(format!(
                "Failed to initialize ExternalEvaluator: {e}"
            ))
        })?;

        let mut state = self.state.lock().await;
        // Another request may have already initialized while we were spawning.
        if let (Some(ref last), Some(ref existing)) = (&state.last_config, &state.testgen) {
            if Arc::ptr_eq(last, &current_config) {
                tracing::debug!(
                    "ExternalEvaluator: testgen-hs already initialized by another request, reusing"
                );
                return Ok(existing.clone());
            }
        }
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
    ) -> Result<TestgenResponse, BlockfrostError> {
        let testgen = self.ensure_testgen().await?;
        let node = node_pool.get();

        /*
         * Prepare txins
         */
        let multi_era_tx = match MultiEraTx::decode(tx_cbor_binary) {
            Ok(tx) => tx,
            Err(err) =>
            // handle pallas decoding error as if it's coming from external binary.
            {
                return Ok(TestgenResponse::Err(serde_json::Value::String(
                    err.to_string(),
                )));
            },
        };

        let txins = extract_inputs(&multi_era_tx)?;

        let utxos_from_node = node.await?.get_utxos_for_txins(txins).await?;

        // Merge utxos from node and user
        let utxos = KeyValuePairs::from_iter(
            additional_utxos
                .into_iter()
                .chain(utxos_from_node.to_vec().into_iter()),
        );

        let utxos_cbor = hex::encode(to_vec(&utxos).map_err(|err| {
            BlockfrostError::internal_server_error(format!(
                "ExternalEvaluator: Failed to serialize UTxOs: {err}"
            ))
        })?);

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
        Ok(response)
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

    pub async fn evaluate_binary_tx_v5(
        &self,
        node_pool: NodePool,
        tx_cbor_binary: &[u8],
        additional_utxos: Option<AdditionalUtxoSet>,
    ) -> Result<serde_json::Value, BlockfrostError> {
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

        let response = self
            .evaluate_binary_tx(node_pool, tx_cbor_binary, user_utxos)
            .await?;
        Ok(wrap_response_v5(response, serde_json::Value::Null))
    }

    pub async fn evaluate_binary_tx_v6(
        &self,
        node_pool: NodePool,
        tx_cbor_binary: &[u8],
        additional_utxos: Option<Vec<AdditionalUtxoV6>>,
    ) -> Result<serde_json::Value, BlockfrostError> {
        let user_utxos = additional_utxos
            .unwrap_or_default()
            .iter()
            .map(
                |utxo| -> Result<(UTxO, TransactionOutput), BlockfrostError> {
                    let txin = UTxO {
                        transaction_id: pallas_crypto::hash::Hash::<32>::from_str(
                            &utxo.transaction.id,
                        )
                        .map_err(|e| {
                            BlockfrostError::custom_400(format!(
                                "invalid transaction id '{}': {e}",
                                utxo.transaction.id
                            ))
                        })?,
                        index: AnyUInt::U64(utxo.index),
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

        let response = self
            .evaluate_binary_tx(node_pool, tx_cbor_binary, user_utxos)
            .await?;
        Ok(wrap_response_v6(response, serde_json::Value::Null))
    }
}

/// Spawn a fresh testgen-hs process and send the init payload.
async fn spawn_and_init_testgen(config: &ChainConfigCache) -> Result<Testgen, AppError> {
    let testgen = Testgen::spawn("evaluate-stream")
        .map_err(|err| AppError::Server(format!("Failed to spawn ExternalEvaluator: {err}")))?;

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

    match testgen.send(payload).await {
        Ok(response) => match response {
            TestgenResponse::Ok(_) => Ok(testgen),
            TestgenResponse::Err(err) => Err(AppError::Server(format!(
                "ExternalEvaluator: Failed to initialize: {err}"
            ))),
        },
        Err(err) => Err(AppError::Server(format!(
            "ExternalEvaluator: Failed to initialize: {err}"
        ))),
    }
}
