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

pub struct TxEvalutionResult {
    pub validator: ValidatorResult,
    pub budget: BudgetResult,
}
#[derive(Serialize)]
pub struct BudgetResult {
    pub memory: u64,
    pub cpu: u64,
}
#[derive(Serialize)]
pub struct ValidatorResult {
    pub purpose: String,
    pub index: u64,
}

pub fn convert_eval_report(pallas_report: EvalReport) -> Vec<TxEvalutionResult> {
    pallas_report.iter().map(convert_eval_result).collect()
}

pub fn convert_eval_result(pallas_result: &TxEvalResult) -> TxEvalutionResult {
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
    pub cbor: String, // @todo can be base16 or base 64 CBOR
    #[serde(rename = "additionalUtxoSet")]
    pub additional_utxo_set: Vec<(TxIn, TxOut)>, //
}

#[derive(Deserialize)]
pub struct TxIn {
    #[serde(rename = "txId")]
    pub tx_id: String,
    pub index: u64,
}

// @todo datumHash, datum and script fields are missing
#[derive(Deserialize)]
pub struct TxOut {
    pub address: String,
    pub value: Value,
    #[serde(rename = "datumHash")]
    pub datum_hash: Option<String>,
    pub datum: Option<Datum>,
    pub script: Option<HashMap<String, Script>>, // script type and details
}

#[derive(Deserialize)]
#[serde(untagged)]
pub enum Datum {
    String(String),
    Map(HashMap<String, String>),
}

#[derive(Deserialize)]
pub struct Value {
    pub coins: u64,
    pub assets: Option<HashMap<String, i64>>, // asset name and number. Asset number can be negative when burning assets
}

// This is missing PlutusV3 since blockfrost uses Ogmios v5.6 which has slightly different data structure
#[derive(Deserialize)]
#[serde(untagged)]
pub enum Script {
    Plutus(String),
    Native(ScriptNative),
}

#[derive(Deserialize)]
pub enum ScriptNative {
    #[serde(rename = "any")]
    Any(Vec<ScriptNative>),
    #[serde(rename = "all")]
    All(Vec<ScriptNative>),
    #[serde(rename = "expiresAt")]
    ExpiresAt(u64),
    #[serde(rename = "startsAt")]
    StartsAt(u64),
    #[serde(untagged)]
    NOf(HashMap<String, Vec<ScriptNative>>),
    #[serde(untagged)]
    String(String),
}
#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn test_json_request() {
        let value = json!({
            "cbor": "enocodedcbor",
            "additionalUtxoSet": [
                [
                  {
                    "txId": "stringstringstringstringstringstringstringstringstringstringstri",
                    "index": 4294967295u64
                  },
                  {
                    "address": "addr_test1qz66ue36465w2qq40005h2hadad6pnjht8mu6sgplsfj74qdjnshguewlx4ww0eet26y2pal4xpav5prcydf28cvxtjqx46x7f",
                    "value": {
                      "coins": 2,
                      "assets": {
                        "3542acb3a64d80c29302260d62c3b87a742ad14abf855ebc6733081e": 42,
                        "b5ae663aaea8e500157bdf4baafd6f5ba0ce5759f7cd4101fc132f54.706174617465": 1337
                      }
                    },
                    "datumHash": "iamadatumhash",
                    "datum": {"key": "value"},
                    "script": {"native": {"all":[{"startsAt": 1234567890},"ec09e5293d384637cd2f004356ef320f3fe3c07030e36bfffe67e2e2",{"1": ["3c07030e36bfff7cd2f004356ef320f3fe3c07030e7cd2f004356437"]}]}}
                  }
                ]
                ]
        });

        let req: TxEvaluationRequest = serde_json::from_value(value).unwrap();
        assert_eq!(req.cbor, "enocodedcbor");
        assert_eq!(req.additional_utxo_set.len(), 1);
        assert_eq!(
            req.additional_utxo_set[0].0.tx_id,
            "stringstringstringstringstringstringstringstringstringstringstri"
        );
        assert_eq!(req.additional_utxo_set[0].0.index, 4294967295u64);
        assert_eq!(
            req.additional_utxo_set[0].1.address,
            "addr_test1qz66ue36465w2qq40005h2hadad6pnjht8mu6sgplsfj74qdjnshguewlx4ww0eet26y2pal4xpav5prcydf28cvxtjqx46x7f"
        );
        assert_eq!(req.additional_utxo_set[0].1.value.coins, 2);
    }
}
