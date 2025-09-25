use std::collections::HashMap;

use pallas_primitives::{Bytes, ExUnits, KeepRaw, conway::RedeemerTag};
use pallas_validate::phase2::EvalReport;
use pallas_validate::phase2::tx::TxEvalResult;
use serde::{Deserialize, Serialize, ser::SerializeMap};

// JSON request
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum TxEvaluationRequest {
    V6(TxEvaluationRequestV6),
    V5Cbor(TxEvaluationRequestV5Cbor),
    V5Evaluate(TxEvaluationRequestV5Evaluate),
}

#[derive(Debug, Deserialize)]
pub struct TxEvaluationRequestV5Cbor {
    pub cbor: String, // @todo can be base16 or base64 CBOR
    #[serde(rename = "additionalUtxoSet")]
    pub additional_utxo_set: Option<AdditionalUtxoSet>,
}

#[derive(Debug, Deserialize)]
pub struct TxEvaluationRequestV5Evaluate {
    pub evaluate: String,
    #[serde(rename = "additionalUtxoSet")]
    pub additional_utxo_set: Option<AdditionalUtxoSet>,
}
// JSON request

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TxEvaluationRequestV6 {
    pub transaction: TransactionCborV6,
    pub additional_utxo: Option<Vec<AdditionalUtxoV6>>,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdditionalUtxoV6 {
    pub transaction: TransactionIdV6,
    // @todo can be base16 or base64 CBOR
    pub index: u64,
    pub address: String,
    pub value: MultiAssetV6,
    #[serde(rename = "datumHash")]
    pub datum_hash: Option<String>,
    pub datum: Option<String>,
    pub script: ScriptV6,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MultiAssetV6 {
    ada: LovelaceV6,
    #[serde(flatten)]
    assets: HashMap<String, serde_json::Value>,
}

/// If language is native, the json structure can be one of cbor or json
/// If language is plutus, only cbor field is expected
///  "script": {
///       "language": "native",
///       "json": {
///         "clause": "signature",
///         "from": "3c07030e36bfff7cd2f004356ef320f3fe3c07030e7cd2f004356437"
///       },
///       "cbor": "string"
///  }
#[derive(Debug, Deserialize)]
#[serde(tag = "language", rename_all = "camelCase")]
pub enum ScriptV6 {
    #[serde(rename = "plutus:v1")]
    PlutusV1 { cbor: String },
    #[serde(rename = "plutus:v2")]
    PlutusV2 { cbor: String },
    #[serde(rename = "plutus:v3")]
    PlutusV3 { cbor: String },
    Native {
        json: Option<ScriptNativeV6>,
        cbor: Option<String>,
    },
}

#[derive(Deserialize, Debug, Clone)]
#[serde(tag = "clause", rename_all = "camelCase")]
pub enum ScriptNativeV6 {
    Signature {
        from: String,
    },
    Any {
        from: Vec<ScriptNativeV6>,
    },
    All {
        from: Vec<ScriptNativeV6>,
    },
    Some {
        at_least: u64,
        from: Vec<ScriptNativeV6>,
    },
    Before {
        slot: u64,
    },
    After {
        slot: u64,
    },
}

#[derive(Deserialize, Debug)]
pub struct TxIn {
    #[serde(rename = "txId")]
    pub tx_id: String,
    pub index: u64,
}

#[derive(Debug, Deserialize)]
pub struct TransactionCborV6 {
    pub cbor: String,
}

#[derive(Debug, Deserialize)]
pub struct TransactionIdV6 {
    pub id: String,
}

#[derive(Debug, Deserialize)]
pub struct LovelaceV6 {
    pub lovelace: u64,
}

#[derive(Deserialize, Debug)]
pub struct TxOut {
    pub address: String,
    pub value: Value,
    #[serde(rename = "datumHash")]
    pub datum_hash: Option<String>,
    pub datum: Option<String>,
    pub script: Option<Script>, // script type and details
}

pub type AdditionalUtxoSet = Vec<(TxIn, TxOut)>;

impl From<Script> for pallas_primitives::conway::ScriptRef<'_> {
    fn from(script: Script) -> Self {
        use pallas_primitives::PlutusScript;
        use pallas_primitives::conway::ScriptRef;
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
                use pallas_primitives::conway::NativeScript;
                let script: NativeScript = script_native.into();
                let r_script: KeepRaw<'_, NativeScript> = script.into(); // @todo: does not generate a valid raw CBOR 
                ScriptRef::NativeScript(r_script)
            },
        }
    }
}

impl From<Script> for pallas_network::miniprotocols::localtxsubmission::primitives::ScriptRef {
    fn from(script: Script) -> Self {
        use pallas_network::miniprotocols::localtxsubmission::primitives::PlutusScript;
        use pallas_network::miniprotocols::localtxsubmission::primitives::ScriptRef;
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
                let script: pallas_network::miniprotocols::localtxsubmission::primitives::NativeScript = script_native.into();
                ScriptRef::NativeScript(script)
            },
        }
    }
}

impl From<ScriptNative>
    for pallas_network::miniprotocols::localtxsubmission::primitives::NativeScript
{
    fn from(script: ScriptNative) -> Self {
        use pallas_network::miniprotocols::localtxsubmission::primitives::NativeScript;
        match script {
            ScriptNative::Any(scripts) => {
                NativeScript::ScriptAny(scripts.into_iter().map(|s| s.into()).collect())
            },
            ScriptNative::All(scripts) => {
                NativeScript::ScriptAll(scripts.into_iter().map(|s| s.into()).collect())
            },
            ScriptNative::ExpiresAt(time) => NativeScript::InvalidHereafter(time),
            ScriptNative::StartsAt(time) => NativeScript::InvalidBefore(time),
            ScriptNative::NOf(h_map) => {
                let (n_str, scripts) = h_map.into_iter().next().unwrap();
                NativeScript::ScriptNOfK(
                    n_str.parse::<u32>().unwrap(),
                    scripts.into_iter().map(|s| s.into()).collect(),
                )
            },
            ScriptNative::Signature(st) => {
                let mut bytes = [0; 28];
                hex::decode_to_slice(st, &mut bytes).unwrap();
                NativeScript::ScriptPubkey(bytes.into())
            },
        }
    }
}

impl From<ScriptNative> for pallas_primitives::conway::NativeScript {
    fn from(script: ScriptNative) -> Self {
        use pallas_primitives::conway::NativeScript;
        match script {
            ScriptNative::Any(scripts) => {
                NativeScript::ScriptAny(scripts.into_iter().map(|s| s.into()).collect())
            },
            ScriptNative::All(scripts) => {
                NativeScript::ScriptAll(scripts.into_iter().map(|s| s.into()).collect())
            },
            ScriptNative::ExpiresAt(time) => NativeScript::InvalidHereafter(time),
            ScriptNative::StartsAt(time) => NativeScript::InvalidBefore(time),
            ScriptNative::NOf(h_map) => {
                let (n_str, scripts) = h_map.into_iter().next().unwrap();
                NativeScript::ScriptNOfK(
                    n_str.parse::<u32>().unwrap(),
                    scripts.into_iter().map(|s| s.into()).collect(),
                )
            },
            ScriptNative::Signature(st) => {
                let mut bytes = [0; 28];
                hex::decode_to_slice(st, &mut bytes).unwrap();
                NativeScript::ScriptPubkey(bytes.into())
            },
        }
    }
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum Datum {
    String(String),
    Map(HashMap<String, String>),
}

#[derive(Deserialize, Debug)]
pub struct Value {
    pub coins: u64,
    pub assets: Option<HashMap<String, u64>>, // asset name and number. Asset number can be negative when burning assets but this behaviour changed in Conway. Now it can be only PositiveCoin
}

// This is originally missing PlutusV3 since blockfrost uses Ogmios v5.6 which has slightly different data structure
#[derive(Deserialize, Debug, Clone)]
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

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub enum ScriptNative {
    Any(Vec<ScriptNative>),
    All(Vec<ScriptNative>),
    ExpiresAt(u64),
    StartsAt(u64),
    #[serde(untagged)]
    NOf(HashMap<String, Vec<ScriptNative>>),
    #[serde(untagged)]
    Signature(String),
}

// JSON response
#[derive(Serialize)]
pub struct TxEvaluationResponse {
    #[serde(rename = "spend:1")]
    pub spend: ExecCost,
    #[serde(rename = "mint:0")]
    pub mint: ExecCost,
}

#[derive(Serialize)]

pub struct ExecCost {
    pub memory: u64,
    pub cpu: u64,
}

// Create a wrapper type for TxEvalResult
pub struct TxEvalResultResponse {
    pub tag: RedeemerTag,
    pub index: u32,
    pub units: ExUnitsResponse,
}

impl From<TxEvalResultResponse> for TxEvalResult {
    fn from(value: TxEvalResultResponse) -> Self {
        // pallas-validate is rapidly evolving. logs and success are new fields, not yet handled.
        TxEvalResult {
            tag: value.tag,
            index: value.index,
            units: ExUnits {
                mem: value.units.memory,
                steps: value.units.cpu,
            },
            logs: Vec::new(),
            success: true,
        }
    }
}

impl From<TxEvalResult> for TxEvalResultResponse {
    fn from(value: TxEvalResult) -> Self {
        TxEvalResultResponse {
            tag: value.tag,
            index: value.index, // @todo fix this in pallas to be u64
            units: ExUnitsResponse {
                memory: value.units.mem,
                cpu: value.units.steps,
            },
        }
    }
}

use serde::ser::Serializer;

impl Serialize for TxEvalResultResponse {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(1))?;

        let tag = match self.tag {
            pallas_primitives::conway::RedeemerTag::Spend => "spend",
            pallas_primitives::conway::RedeemerTag::Mint => "mint",
            pallas_primitives::conway::RedeemerTag::Cert => "cert",
            pallas_primitives::conway::RedeemerTag::Reward => "reward",
            pallas_primitives::conway::RedeemerTag::Vote => "vote",
            pallas_primitives::conway::RedeemerTag::Propose => "propose",
        };

        let tag = format!("{}:{}", tag, self.index);
        map.serialize_entry(&tag, &self.units)?;
        map.end()
    }
}

#[derive(Serialize)]
//#[serde(remote = "ExUnits")]
pub struct ExUnitsResponse {
    memory: u64,
    cpu: u64,
}

impl From<ExUnitsResponse> for ExUnits {
    fn from(value: ExUnitsResponse) -> Self {
        ExUnits {
            mem: value.memory,
            steps: value.cpu,
        }
    }
}

impl From<ExUnits> for ExUnitsResponse {
    fn from(value: ExUnits) -> Self {
        ExUnitsResponse {
            memory: value.mem,
            cpu: value.steps,
        }
    }
}

pub fn convert_eval_report(pallas_report: EvalReport) -> Vec<TxEvalResultResponse> {
    pallas_report
        .into_iter()
        .map(|pallas_result| pallas_result.into())
        .collect()
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

        let req: TxEvaluationRequestV5Cbor = serde_json::from_value(value).unwrap();
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
    fn test_json_request_native_mof() {
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
                    "script": {"native": {"2":[{"startsAt": 1234567890u64},"ec09e5293d384637cd2f004356ef320f3fe3c07030e36bfffe67e2e2","3c07030e36bfff7cd2f004356ef320f3fe3c07030e7cd2f004356437"]}}
                  }
                ]
                ]
        });

        let req: TxEvaluationRequestV5Cbor = serde_json::from_value(value).unwrap();
        assert_eq!(req.cbor, "enocodedcbor");
        let utxo_set = req.additional_utxo_set.unwrap();
        match &utxo_set[0].1.script {
            Some(Script::Native(script_native)) => match script_native {
                ScriptNative::NOf(h_map) => {
                    let (k, _v) = h_map.iter().next().unwrap();
                    assert_eq!(k, "2");
                },
                _ => panic!("Expected ScriptNative::NOf"),
            },
            _ => panic!("Expected Script::Native"),
        }
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

    #[test]
    fn test_json_request_native_v6() {
        let value = json!({
          "transaction": {
            "cbor": "transaction-cbor"
          },
          "additionalUtxo": [
            {
              "transaction": {
                "id": "stringstringstringstringstringstringstringstringstringstringstri"
              },
              "index": 4294967295u64,
              "address": "addr1q9d34spgg2kdy47n82e7x9pdd6vql6d2engxmpj20jmhuc2047yqd4xnh7u6u5jp4t0q3fkxzckph4tgnzvamlu7k5psuahzcp",
              "value": {
                "ada": {
                  "lovelace": 3
                },
                "property1": {
                  "property1": 0,
                  "property2": 0
                },
                "property2": {
                  "property1": 0,
                  "property2": 0
                }
              },
              "datumHash": "c248757d390181c517a5beadc9c3fe64bf821d3e889a963fc717003ec248757d",
              "datum": "string",
              "script": {
                "language": "native",
                "json": {
                  "clause": "signature",
                  "from": "3c07030e36bfff7cd2f004356ef320f3fe3c07030e7cd2f004356437"
                },
                "cbor": "string"
              }
            }
          ]
        });

        let req: TxEvaluationRequestV6 = serde_json::from_value(value).unwrap();

        assert_eq!(req.transaction.cbor, "transaction-cbor");

        let utxo_set = req.additional_utxo.unwrap();

        assert_eq!(utxo_set.len(), 1);
        assert_eq!(
            utxo_set[0].transaction.id,
            "stringstringstringstringstringstringstringstringstringstringstri"
        );
        assert_eq!(utxo_set[0].index, 4294967295u64);
        assert_eq!(
            utxo_set[0].address,
            "addr1q9d34spgg2kdy47n82e7x9pdd6vql6d2engxmpj20jmhuc2047yqd4xnh7u6u5jp4t0q3fkxzckph4tgnzvamlu7k5psuahzcp"
        );
        assert_eq!(utxo_set[0].value.ada.lovelace, 3);
        assert_eq!(utxo_set[0].value.assets.len(), 2);
    }

    #[test]
    fn test_json_request_versions() {
        let version5_cbor = json!({
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

        let version5_evaluate = json!({
            "evaluate": "enocodedcbor",
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

        let version6 = json!({
          "transaction": {
            "cbor": "transaction-cbor"
          },
          "additionalUtxo": [
            {
              "transaction": {
                "id": "stringstringstringstringstringstringstringstringstringstringstri"
              },
              "index": 4294967295u64,
              "address": "addr1q9d34spgg2kdy47n82e7x9pdd6vql6d2engxmpj20jmhuc2047yqd4xnh7u6u5jp4t0q3fkxzckph4tgnzvamlu7k5psuahzcp",
              "value": {
                "ada": {
                  "lovelace": 3
                },
                "property1": {
                  "property1": 0,
                  "property2": 0
                },
                "property2": {
                  "property1": 0,
                  "property2": 0
                }
              },
              "datumHash": "c248757d390181c517a5beadc9c3fe64bf821d3e889a963fc717003ec248757d",
              "datum": "string",
              "script": {
                "language": "native",
                "json": {
                  "clause": "signature",
                  "from": "3c07030e36bfff7cd2f004356ef320f3fe3c07030e7cd2f004356437"
                },
                "cbor": "string"
              }
            }
          ]
        });

        let v5_cbor: TxEvaluationRequest = serde_json::from_value(version5_cbor).unwrap();
        let v5_evaluate: TxEvaluationRequest = serde_json::from_value(version5_evaluate).unwrap();
        let v6: TxEvaluationRequest = serde_json::from_value(version6).unwrap();

        match v5_cbor {
            TxEvaluationRequest::V6(_) => panic!("Expected V5 request, got V6"),
            TxEvaluationRequest::V5Evaluate(_) => {
                panic!("Expected V5 cbor request, got V5 evaluate")
            },
            TxEvaluationRequest::V5Cbor(_) => (),
        }

        match v5_evaluate {
            TxEvaluationRequest::V6(_) => panic!("Expected V5 request, got V6"),
            TxEvaluationRequest::V5Evaluate(_) => (),
            TxEvaluationRequest::V5Cbor(_) => panic!("Expected V5 evaluate request, got V5 cbor"),
        }

        match v6 {
            TxEvaluationRequest::V6(_) => (),
            TxEvaluationRequest::V5Evaluate(_) => {
                panic!("Expected V5 cbor request, got V5 evaluate")
            },
            TxEvaluationRequest::V5Cbor(_) => panic!("Expected V6 request, got V5 cbor"),
        }
    }
}
