use std::collections::HashMap;

use crate::node::sync_progress::NodeInfo;
use pallas_primitives::{
    Bytes, KeepRaw, PlutusScript,
    conway::{NativeScript, ScriptRef},
};
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
#[derive(Serialize, Debug)]
pub struct TxEvalutionResult {
    pub validator: ValidatorResult,
    pub budget: BudgetResult,
}
#[derive(Serialize, Debug)]
pub struct BudgetResult {
    pub memory: u64,
    pub cpu: u64,
}
#[derive(Serialize, Debug)]
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
    pub cbor: String, // @todo can be base16 or base64 CBOR
    #[serde(rename = "additionalUtxoSet")]
    pub additional_utxo_set: Option<AdditionalUtxoSet>,
}

pub type AdditionalUtxoSet = Vec<(TxIn, TxOut)>;
#[derive(Deserialize)]
pub struct TxIn {
    #[serde(rename = "txId")]
    pub tx_id: String,
    pub index: u64,
}
#[derive(Deserialize)]
pub struct TxOut {
    pub address: String,
    pub value: Value,
    #[serde(rename = "datumHash")]
    pub datum_hash: Option<String>,
    pub datum: Option<String>,
    pub script: Option<Script>, // script type and details
}

impl From<Script> for ScriptRef<'_> {
    fn from(script: Script) -> Self {
        match script {
            Script::PlutusV1(s) => {
                ScriptRef::PlutusV1Script(PlutusScript::<1>(Bytes::from(hex::decode(s).unwrap())))
            },
            Script::PlutusV2(s) => {
                ScriptRef::PlutusV2Script(PlutusScript::<2>(Bytes::from(hex::decode(s).unwrap())))
            },
            Script::PlutusV3(s) => {
                ScriptRef::PlutusV3Script(PlutusScript::<3>(Bytes::from(hex::decode(s).unwrap())))
            },
            Script::Native(script_native) => {
                let script: NativeScript = script_native.into();
                let r_script: KeepRaw<'_, NativeScript> = script.into(); // @todo: does not generate a valid raw CBOR 
                ScriptRef::NativeScript(r_script)
            },
        }
    }
}

impl From<ScriptNative> for NativeScript {
    fn from(script: ScriptNative) -> Self {
        match script {
            ScriptNative::Any(scripts) => {
                NativeScript::ScriptAny(scripts.into_iter().map(|s| s.into()).collect())
            },
            ScriptNative::All(scripts) => {
                NativeScript::ScriptAll(scripts.into_iter().map(|s| s.into()).collect())
            },
            ScriptNative::ExpiresAt(time) => NativeScript::InvalidHereafter(time),
            ScriptNative::StartsAt(time) => NativeScript::InvalidBefore(time),
            ScriptNative::NOf(n, scripts) => {
                NativeScript::ScriptNOfK(n, scripts.into_iter().map(|s| s.into()).collect())
            },
            ScriptNative::String(st) => {
                let mut bytes = [0; 28];
                hex::decode_to_slice(st, &mut bytes).unwrap();
                NativeScript::ScriptPubkey(bytes.into())
            },
        }
    }
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
    pub assets: Option<HashMap<String, u64>>, // asset name and number. Asset number can be negative when burning assets but this behaviour changed in Conway. Now it can be only PositiveCoin
}

// This is originally missing PlutusV3 since blockfrost uses Ogmios v5.6 which has slightly different data structure
#[derive(Deserialize)]
//#[serde(untagged)]
pub enum Script {
    #[serde(rename = "plutus:v1")]
    PlutusV1(String),
    #[serde(rename = "plutus:v2")]
    PlutusV2(String),
    #[serde(rename = "plutus:v3")]
    PlutusV3(String),
    #[serde(rename = "native")]
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
    #[serde(rename = "NOf")]
    NOf(u32, Vec<ScriptNative>),
    #[serde(untagged)]
    String(String),
}
#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn test_json_request_native() {
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
                    "datum": "base16datum",
                    "script": {"native": {"all":[{"startsAt": 1234567890u64},"ec09e5293d384637cd2f004356ef320f3fe3c07030e36bfffe67e2e2","3c07030e36bfff7cd2f004356ef320f3fe3c07030e7cd2f004356437"]}}
                  }
                ]
                ]
        });

        let req: TxEvaluationRequest = serde_json::from_value(value).unwrap();
        assert_eq!(req.cbor, "enocodedcbor");
        let utxo_set = req.additional_utxo_set.unwrap();
        assert_eq!(utxo_set.len(), 1);
        assert_eq!(
            utxo_set[0].0.tx_id,
            "stringstringstringstringstringstringstringstringstringstringstri"
        );
        assert_eq!(utxo_set[0].0.index, 4294967295u64);
        assert_eq!(
            utxo_set[0].1.address,
            "addr_test1qz66ue36465w2qq40005h2hadad6pnjht8mu6sgplsfj74qdjnshguewlx4ww0eet26y2pal4xpav5prcydf28cvxtjqx46x7f"
        );
        assert_eq!(utxo_set[0].1.value.coins, 2);
    }

    #[test]
    fn test_json_request_plutusv2() {
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
                    "datum": "base16datum",
                    "script": {"plutus:v2": "base16script"}
                  }
                ]
                ]
        });

        let _req: TxEvaluationRequest = serde_json::from_value(value).unwrap();
    }
}
