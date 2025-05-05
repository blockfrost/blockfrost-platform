pub mod root;
pub mod utxos;

use std::collections::HashMap;

use crate::node::sync_progress::NodeInfo;
use pallas_validate::phase2::EvalReport;
use pallas_validate::phase2::tx::TxEvalResult;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub struct EvaluateResponse {
    pub name: String,
    pub version: String,
    pub revision: String,
    pub healthy: bool,
    pub node_info: Option<NodeInfo>,
    pub errors: Vec<String>,
}

// JSON response
#[derive(Serialize)]

struct TxEvalutionResult {
    validator: ValidatorResult,
    budget: BudgetResult,
}
#[derive(Serialize)]
struct BudgetResult {
    memory: u64,
    cpu: u64,
}
#[derive(Serialize)]
struct ValidatorResult {
    purpose: String,
    index: u64,
}

fn convert_eval_report(pallas_report: EvalReport) -> Vec<TxEvalutionResult> {
    pallas_report.iter().map(convert_eval_result).collect()
}

fn convert_eval_result(pallas_result: &TxEvalResult) -> TxEvalutionResult {
    use pallas_primitives::conway::RedeemerTag::*;
    TxEvalutionResult {
        validator: ValidatorResult {
            purpose: match pallas_result.tag {
                Spend => "spend".to_string(),
                Mint => "mint".to_string(),
                Cert => "publish".to_string(),
                Reward => "withdraw".to_string(),
                Vote => "vote".to_string(),
                Propose => "propose".to_string(),
            },
            index: pallas_result.index as u64, // @todo fix this in pallas to be u64
        },
        budget: BudgetResult {
            memory: pallas_result.units.mem,
            cpu: pallas_result.units.steps,
        },
    }
}

// JSON request

#[derive(Deserialize)]
pub struct TxEvaluationRequest {
    cbor: String, // base16-encoded CBOR
    #[serde(rename = "additionalUtxoSet")]
    additional_utxo_set: String, //
}

#[derive(Deserialize)]
pub struct TxIn {
    #[serde(rename = "txId")]
    tx_id: String,
    index: u32,
}

// @todo datumHash, datum and script fields are missing
#[derive(Deserialize)]
pub struct TxOut {
    address: String,
    value: Value,
    #[serde(rename = "datumHash")]
    datum_hash: Option<String>,
    datum: Option<Datum>,
}
#[derive(Deserialize)]
pub enum Datum {
    String(String),
    Map(HashMap<String, String>),
}

#[derive(Deserialize)]
pub struct Value {
    coins: Quantity,
    assets: Option<HashMap<String, Quantity>>,
}

#[derive(Deserialize)]
pub enum Quantity {
    Num(u64),
    String(String),
}
