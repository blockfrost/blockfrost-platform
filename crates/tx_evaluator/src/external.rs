use std::str::FromStr;

use node::{chain_config::{ChainConfigCache, SlotConfig}, pool::NodePool};
use pallas_codec::{
    minicbor::to_vec,
    utils::{AnyUInt, CborWrap},
};

use pallas_network::miniprotocols::{
    localstate::queries_v16::{self, PostAlonsoTransactionOutput, TransactionOutput, TxIns, UTxO},
    localtxsubmission::primitives::ScriptRef,
};
use pallas_primitives::{KeyValuePairs, TransactionInput, byron};
use pallas_traverse::MultiEraTx;
use serde::Serialize;

use common::errors::{AppError, BlockfrostError};
use testgen::testgen::Testgen;

use crate::{model::AdditionalUtxoSet, native::{convert_to_datum_option_network, convert_to_network_value, create_address}};

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
        let testgen =  testgen::testgen::Testgen::spawn("evaluate-stream")
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

        (self.testgen.send(payload).await).map_err(|err| {
            AppError::Server(format!("ExternalEvaluator: Failed to initialize: {err}"))
        })
    }

    fn convert_alonzo_txin(txin: &TransactionInput) -> queries_v16::TransactionInput {
        queries_v16::TransactionInput {
            transaction_id: txin.transaction_id,
            index: txin.index,
        }
    }

    fn convert_byron_txin(txin: &byron::TxIn) -> queries_v16::TransactionInput {
        match txin {
            byron::TxIn::Variant0(CborWrap((tx, idx))) => queries_v16::TransactionInput {
                transaction_id: *tx,
                index: *idx as u64,
            },
            _ => unreachable!(),
        }
    }

    fn extract_inputs(tx: MultiEraTx) -> TxIns {
        let txins = match tx {
            MultiEraTx::AlonzoCompatible(x, _) => x
                .transaction_body
                .inputs
                .iter()
                .map(Self::convert_alonzo_txin)
                .collect(),
            MultiEraTx::Babbage(x) => x
                .transaction_body
                .inputs
                .iter()
                .map(Self::convert_alonzo_txin)
                .collect(),
            MultiEraTx::Byron(x) => x
                .transaction
                .inputs
                .iter()
                .map(Self::convert_byron_txin)
                .collect(),
            MultiEraTx::Conway(x) => x
                .transaction_body
                .inputs
                .iter()
                .map(Self::convert_alonzo_txin)
                .collect(),
            _ => unreachable!("unknown era transaction"),
        };
        txins
    }

    pub async fn evaluate_binary_tx(
        &self,
        node_pool: NodePool,
        tx_cbor_binary: &[u8],
        additional_utxos: Option<AdditionalUtxoSet>,
    ) -> Result<serde_json::Value, BlockfrostError> {
        let mut node = node_pool.get().await?;

        /*
         * Prepare txins
         */
        let multi_era_tx = MultiEraTx::decode(tx_cbor_binary).unwrap();
        let txins = Self::extract_inputs(multi_era_tx);

        let utxos_from_node = node.get_utxos_for_txins(txins).await?;

        let incoming_user_utxos = additional_utxos.unwrap_or_default();

        let user_utxos = incoming_user_utxos.iter().map(|(utxo, tout)| {
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

            let value = convert_to_network_value(&tout.value);

            let txout = TransactionOutput::Current(PostAlonsoTransactionOutput {
                address,
                script_ref,
                amount: value,
                inline_datum,
            });

            (txin, txout)
        });

        // Merge utxos from node and user
        let utxos =
            KeyValuePairs::from_iter(user_utxos.chain(utxos_from_node.to_vec().into_iter()));

        let utxos_cbor = hex::encode(to_vec(&utxos).map_err(|err| {
            BlockfrostError::internal_server_error(format!("ExternalEvaluator: Failed to serialize UTxOs: {err}"))
        })?);

        let payload = EvalPayload {
            tx: hex::encode(tx_cbor_binary),
            utxos: utxos_cbor,
        };

        let json = serde_json::to_string(&payload).map_err(|err| {
            BlockfrostError::internal_server_error(format!("ExternalEvaluator: Failed to serialize payload: {err}"))
        })?;

        let response = self.testgen.send(json).await.map_err(|err| {
            BlockfrostError::internal_server_error(format!("ExternalEvaluator: Failed to send payload: {err}"))
        })?;

        Ok(response)
    }
}
