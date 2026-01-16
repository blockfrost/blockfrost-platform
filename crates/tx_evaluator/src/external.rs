use std::str::FromStr;

use bf_node::pool::NodePool;
use pallas_codec::{
    minicbor::to_vec,
    utils::{AnyUInt, CborWrap},
};

use pallas_network::miniprotocols::{
    localstate::queries_v16::{PostAlonsoTransactionOutput, TransactionOutput, UTxO},
    localtxsubmission::primitives::ScriptRef,
};
use pallas_primitives::KeyValuePairs;
use pallas_traverse::MultiEraTx;
use serde::Serialize;

use bf_common::{
    chain_config::{ChainConfigCache, SlotConfig},
    errors::{AppError, BlockfrostError},
};
use bf_testgen::testgen::{Testgen, TestgenResponse};

use crate::{
    model::api::{AdditionalUtxoSet, AdditionalUtxoV6},
    native::{
        convert_to_datum_option_network, convert_to_network_value, convert_to_network_value_v6,
        create_address, extract_inputs,
    },
    wrapper::{wrap_response_v5, wrap_response_v6},
};

#[derive(Clone)]
pub struct ExternalEvaluator {
    testgen: Testgen,
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

/// Evaluates the given tx with utxos using the external testgen exe, which is a Haskell binary.
impl ExternalEvaluator {
    /// Spawn testgen with specific command 'evaluate-stream'
    pub async fn spawn(config: ChainConfigCache) -> Result<Self, AppError> {
        let testgen = Testgen::spawn("evaluate-stream")
            .map_err(|err| AppError::Server(format!("Failed to spawn ExternalEvaluator: {err}")))?;

        let evaluator = Self { testgen };
        evaluator.init(config).await?;

        Ok(evaluator)
    }

    /// Sends repeatative data as the first communication so we don't need to send every time.
    /// Also makes sure the child processes behaves as expected.
    async fn init(&self, config: ChainConfigCache) -> Result<serde_json::Value, AppError> {
        use pallas_codec::minicbor::to_vec;

        let system_start = to_vec(config.genesis_config.system_start).map_err(|err| {
            AppError::Server(format!(
                "ExternalEvaluator: failed to serialize genesis config: {err}"
            ))
        })?;

        let protocol_params = to_vec(config.protocol_params).map_err(|err| {
            AppError::Server(format!(
                "ExternalEvaluator: failed to serialize protocol params: {err}"
            ))
        })?;

        let init_payload = InitPayload {
            system_start: hex::encode(system_start),
            protocol_params: hex::encode(protocol_params),
            slot_config: config.slot_config,
            era: config.era,
        };

        let payload = serde_json::to_string(&init_payload).map_err(|err| {
            AppError::Server(format!(
                "ExternalEvaluator: failed to serialize initial payload: {err}"
            ))
        })?;

        /*
        (self.testgen.send(payload).await).map_err(|err| {
            AppError::Server(format!("ExternalEvaluator: Failed to initialize: {err}"))
        })
        */
        // println!("1 -> {}", payload);
        match self.testgen.send(payload).await {
            Ok(response) => match response {
                TestgenResponse::Ok(value) => Ok(value),
                TestgenResponse::Err(err) => Err(AppError::Server(format!(
                    "ExternalEvaluator: Failed to initialize: {err}"
                ))),
            },
            Err(err) => Err(AppError::Server(format!(
                "ExternalEvaluator: Failed to initialize: {err}"
            ))),
        }
    }

    pub async fn evaluate_binary_tx(
        &self,
        node_pool: NodePool,
        tx_cbor_binary: &[u8],
        additional_utxos: Vec<(UTxO, TransactionOutput)>,
    ) -> Result<TestgenResponse, BlockfrostError> {
        let node = node_pool.get();

        /*
         * Prepare txins
         */
        let multi_era_tx = match MultiEraTx::decode(tx_cbor_binary) {
            Ok(tx) => tx,
            Err(err) =>
            // handle pallas decoding error as if it's coming from external binary.
            {
                return Ok(TestgenResponse::Err(
                    serde_json::to_value(err.to_string()).unwrap(),
                ));
            },
        };

        let txins = extract_inputs(multi_era_tx);

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

        let response = self.testgen.send(json).await.map_err(|err| {
            BlockfrostError::internal_server_error(format!(
                "ExternalEvaluator: Failed to send payload: {err}"
            ))
        })?;
        Ok(response)
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
            .map(|(utxo, tout)| {
                let inline_datum = convert_to_datum_option_network(&tout.datum);

                let txin = UTxO {
                    transaction_id: pallas_crypto::hash::Hash::<32>::from_str(&utxo.tx_id).unwrap(),
                    index: AnyUInt::U64(utxo.index),
                };

                // A Cardano address (either legacy format or new format).
                let address = create_address(&tout.address);

                let script_ref: Option<CborWrap<ScriptRef>> = tout.script.as_ref().map(|script| {
                    let script_ref = ScriptRef::from(script.clone());
                    CborWrap(script_ref)
                });

                let amount = convert_to_network_value(&tout.value);

                let txout = TransactionOutput::Current(PostAlonsoTransactionOutput {
                    address,
                    script_ref,
                    amount,
                    inline_datum,
                });

                (txin, txout)
            })
            .collect();

        let response = self
            .evaluate_binary_tx(node_pool, tx_cbor_binary, user_utxos)
            .await
            .unwrap();
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
            .map(|utxo| {
                let inline_datum = convert_to_datum_option_network(&utxo.datum);

                let txin = UTxO {
                    transaction_id: pallas_crypto::hash::Hash::<32>::from_str(&utxo.transaction.id)
                        .unwrap(),
                    index: AnyUInt::U64(utxo.index),
                };

                // A Cardano address (either legacy format or new format).
                let address = create_address(&utxo.address);

                let script_ref: Option<CborWrap<ScriptRef>> = utxo.script.as_ref().map(|script| {
                    let script_ref = ScriptRef::from(script);
                    CborWrap(script_ref)
                });

                let amount = convert_to_network_value_v6(&utxo.value);

                let txout = TransactionOutput::Current(PostAlonsoTransactionOutput {
                    address,
                    script_ref,
                    amount,
                    inline_datum,
                });

                (txin, txout)
            })
            .collect();

        let response = self
            .evaluate_binary_tx(node_pool, tx_cbor_binary, user_utxos)
            .await
            .unwrap();
        Ok(wrap_response_v6(response, serde_json::Value::Null))
    }
}
