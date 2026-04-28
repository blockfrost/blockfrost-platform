use core::fmt;
use std::collections::HashMap;
use std::str::FromStr;

use bf_common::errors::BlockfrostError;
use pallas_primitives::{Bytes, ExUnits, KeepRaw, conway::RedeemerTag};
use pallas_validate::phase2::EvalReport;
use pallas_validate::phase2::tx::TxEvalResult;
use serde::de;
use serde::{Deserialize, Serialize, ser::SerializeMap};

fn decode_script_hex(s: &str) -> Result<Bytes, BlockfrostError> {
    hex::decode(s)
        .map(Bytes::from)
        .map_err(|e| BlockfrostError::custom_400(format!("invalid hex-encoded script CBOR: {e}")))
}

fn decode_pubkey_hash(st: &str) -> Result<[u8; 28], BlockfrostError> {
    let mut bytes = [0u8; 28];
    hex::decode_to_slice(st, &mut bytes).map_err(|e| {
        BlockfrostError::custom_400(format!("invalid hex-encoded pubkey hash: {e}"))
    })?;
    Ok(bytes)
}

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
    pub mirror: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct TxEvaluationRequestV5Evaluate {
    pub evaluate: String,
    #[serde(rename = "additionalUtxoSet")]
    pub additional_utxo_set: Option<AdditionalUtxoSet>,
    pub mirror: Option<serde_json::Value>,
}
// JSON request

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TxEvaluationRequestV6 {
    pub transaction: TransactionCborV6,
    pub additional_utxo: Option<Vec<AdditionalUtxoV6>>,
    #[serde(default)]
    pub mirror: Option<serde_json::Value>,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdditionalUtxoV6 {
    #[serde(flatten)]
    pub output_reference: OutputReferenceV6,
    pub address: String,
    pub value: ValueV6,
    #[serde(rename = "datumHash")]
    pub datum_hash: Option<String>,
    pub datum: Option<String>,
    pub script: Option<ScriptV6>,
}

impl AdditionalUtxoV6 {
    pub fn transaction_id(&self) -> &str {
        &self.output_reference.transaction.id
    }

    pub fn output_index(&self) -> u64 {
        self.output_reference.index
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValueV6 {
    pub ada: LovelaceV6,
    #[serde(flatten)]
    pub assets: HashMap<String, HashMap<String, u64>>,
}

/// If language is native, the json structure can be one of cbor or json.
/// If language is plutus, only cbor field is expected.
///
/// ```json
/// {
///     "script": {
///         "language": "native",
///         "json": {
///             "clause": "signature",
///             "from": "3c07030e36bfff7cd2f004356ef320f3fe3c07030e7cd2f004356437"
///         },
///         "cbor": "string"
///     }
/// }
/// ```
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
        at_least: u32,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TransactionIdV6 {
    pub id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OutputReferenceV6 {
    pub transaction: TransactionIdV6,
    pub index: u64,
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
#[derive(Serialize, Debug)]
pub struct OgmiosError {
    pub code: i64,
    pub message: String,
    pub data: EvaluationError,
}

impl OgmiosError {
    pub fn deserialization_error(err: String) -> OgmiosError {
        Self {
            code: -32602,
            message: "Invalid transaction; It looks like the given transaction wasn't well-formed. Note that I try to decode the transaction in only Conway era, and copy the error into others."
                .to_string(),
            data: EvaluationError::Deserialization(DeserializationErrorData::conway_only(err)),
        }
    }

    pub fn incompatible_era(era: String) -> OgmiosError {
        Self {
            code: 3000,
            message: "Trying to evaluate a transaction from an old era (prior to Alonzo)."
                .to_string(),
            data: EvaluationError::Evaluation(serde_json::json!({ "incompatibleEra": era })),
        }
    }

    pub fn invalid_request(msg: String) -> OgmiosError {
        Self {
            code: -32600,
            data: EvaluationError::Evaluation(json!(null)),
            message: msg,
        }
    }

    pub fn overlapping_utxo(refs: Vec<OutputReferenceV6>) -> OgmiosError {
        Self {
            code: 3002,
            message: "Some user-provided additional UTxO entries overlap with those that exist in the ledger."
                .to_string(),
            data: EvaluationError::Evaluation(json!({ "overlappingOutputReferences": refs })),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum EvaluationError {
    Evaluation(serde_json::Value),
    Deserialization(DeserializationErrorData),
}

/// Serialization produces a JSON object with 6 era fields (matching Ogmios response format).
/// The custom `Deserialize` impl accepts a single string and copies it into all 6 eras
/// via `conway_only()`, since we only decode in Conway era. This means serialization and
/// deserialization are not invertible by design: we receive a plain error string from our
/// internal deserialization path but serve the expanded multi-era format to API consumers.
#[derive(Serialize, Debug)]
pub struct DeserializationErrorData {
    pub shelley: String,
    pub allegra: String,
    pub mary: String,
    pub alonzo: String,
    pub babbage: String,
    pub conway: String,
}

impl DeserializationErrorData {
    /// Copy conway error into all eras (see [`DeserializationErrorData`] for rationale).
    pub fn conway_only(err: String) -> Self {
        Self {
            shelley: err.clone(),
            allegra: err.clone(),
            mary: err.clone(),
            alonzo: err.clone(),
            babbage: err.clone(),
            conway: err,
        }
    }
}

impl<'de> Deserialize<'de> for DeserializationErrorData {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Visitor;

        struct DeserializationErrorDataVisitor;
        impl Visitor<'_> for DeserializationErrorDataVisitor {
            type Value = DeserializationErrorData;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a string representing a DeserializationErrorData")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(DeserializationErrorData::conway_only(v.to_string()))
            }
        }

        deserializer.deserialize_str(DeserializationErrorDataVisitor)
    }
}

impl TryFrom<Script> for pallas_primitives::conway::ScriptRef<'_> {
    type Error = BlockfrostError;

    fn try_from(script: Script) -> Result<Self, Self::Error> {
        use pallas_primitives::PlutusScript;
        use pallas_primitives::conway::ScriptRef;
        Ok(match script {
            Script::PlutusV1(s) => {
                ScriptRef::PlutusV1Script(PlutusScript::<1>(decode_script_hex(&s)?))
            },
            Script::PlutusV2(s) => {
                ScriptRef::PlutusV2Script(PlutusScript::<2>(decode_script_hex(&s)?))
            },
            Script::PlutusV3(s) => {
                ScriptRef::PlutusV3Script(PlutusScript::<3>(decode_script_hex(&s)?))
            },
            Script::Native(script_native) => {
                use pallas_primitives::conway::NativeScript;
                let script: NativeScript = script_native.try_into()?;
                let r_script: KeepRaw<'_, NativeScript> = script.into(); // @todo: does not generate a valid raw CBOR
                ScriptRef::NativeScript(r_script)
            },
        })
    }
}

impl TryFrom<Script> for pallas_network::miniprotocols::localtxsubmission::primitives::ScriptRef {
    type Error = BlockfrostError;

    fn try_from(script: Script) -> Result<Self, Self::Error> {
        use pallas_network::miniprotocols::localtxsubmission::primitives::{
            PlutusScript, ScriptRef,
        };
        Ok(match script {
            Script::PlutusV1(s) => {
                ScriptRef::PlutusV1Script(PlutusScript::<1>(decode_script_hex(&s)?))
            },
            Script::PlutusV2(s) => {
                ScriptRef::PlutusV2Script(PlutusScript::<2>(decode_script_hex(&s)?))
            },
            Script::PlutusV3(s) => {
                ScriptRef::PlutusV3Script(PlutusScript::<3>(decode_script_hex(&s)?))
            },
            Script::Native(script_native) => {
                let script: pallas_network::miniprotocols::localtxsubmission::primitives::NativeScript =
                    script_native.try_into()?;
                ScriptRef::NativeScript(script)
            },
        })
    }
}

impl TryFrom<&ScriptV6>
    for pallas_network::miniprotocols::localtxsubmission::primitives::ScriptRef
{
    type Error = BlockfrostError;

    fn try_from(script: &ScriptV6) -> Result<Self, Self::Error> {
        use pallas_network::miniprotocols::localtxsubmission::primitives::{
            PlutusScript, ScriptRef,
        };
        Ok(match script {
            ScriptV6::PlutusV1 { cbor } => {
                ScriptRef::PlutusV1Script(PlutusScript::<1>(decode_script_hex(cbor)?))
            },
            ScriptV6::PlutusV2 { cbor } => {
                ScriptRef::PlutusV2Script(PlutusScript::<2>(decode_script_hex(cbor)?))
            },
            ScriptV6::PlutusV3 { cbor } => {
                ScriptRef::PlutusV3Script(PlutusScript::<3>(decode_script_hex(cbor)?))
            },
            ScriptV6::Native { cbor, json } => match cbor.as_deref() {
                Some(c) => ScriptRef::NativeScript(
                    pallas_codec::minicbor::decode(&decode_script_hex(c)?).map_err(|e| {
                        BlockfrostError::custom_400(format!("invalid CBOR in native script: {e}"))
                    })?,
                ),
                None => ScriptRef::NativeScript(
                    json.clone()
                        .ok_or_else(|| {
                            BlockfrostError::custom_400(
                                "ScriptV6::Native: neither cbor nor json provided".to_string(),
                            )
                        })?
                        .try_into()?,
                ),
            },
        })
    }
}

impl TryFrom<ScriptNative>
    for pallas_network::miniprotocols::localtxsubmission::primitives::NativeScript
{
    type Error = BlockfrostError;

    fn try_from(script: ScriptNative) -> Result<Self, Self::Error> {
        use pallas_network::miniprotocols::localtxsubmission::primitives::NativeScript;
        Ok(match script {
            ScriptNative::Any(scripts) => NativeScript::ScriptAny(
                scripts
                    .into_iter()
                    .map(|s| s.try_into())
                    .collect::<Result<_, _>>()?,
            ),
            ScriptNative::All(scripts) => NativeScript::ScriptAll(
                scripts
                    .into_iter()
                    .map(|s| s.try_into())
                    .collect::<Result<_, _>>()?,
            ),
            ScriptNative::ExpiresAt(time) => NativeScript::InvalidHereafter(time),
            ScriptNative::StartsAt(time) => NativeScript::InvalidBefore(time),
            ScriptNative::NOf(h_map) => {
                let (n_str, scripts) = h_map
                    .into_iter()
                    .next()
                    .ok_or_else(|| BlockfrostError::custom_400("NOf: empty map".to_string()))?;
                NativeScript::ScriptNOfK(
                    n_str.parse::<u32>().map_err(|e| {
                        BlockfrostError::custom_400(format!("NOf: invalid n key: {e}"))
                    })?,
                    scripts
                        .into_iter()
                        .map(|s| s.try_into())
                        .collect::<Result<_, _>>()?,
                )
            },
            ScriptNative::Signature(st) => {
                NativeScript::ScriptPubkey(decode_pubkey_hash(&st)?.into())
            },
        })
    }
}

impl TryFrom<ScriptNativeV6>
    for pallas_network::miniprotocols::localtxsubmission::primitives::NativeScript
{
    type Error = BlockfrostError;

    fn try_from(script: ScriptNativeV6) -> Result<Self, Self::Error> {
        use ScriptNativeV6::*;
        use pallas_network::miniprotocols::localtxsubmission::primitives::NativeScript;
        Ok(match script {
            Signature { from } => NativeScript::ScriptPubkey(decode_pubkey_hash(&from)?.into()),
            Any { from } => NativeScript::ScriptAny(
                from.into_iter()
                    .map(|s| s.try_into())
                    .collect::<Result<_, _>>()?,
            ),
            All { from } => NativeScript::ScriptAll(
                from.into_iter()
                    .map(|s| s.try_into())
                    .collect::<Result<_, _>>()?,
            ),
            Some { at_least, from } => NativeScript::ScriptNOfK(
                at_least,
                from.into_iter()
                    .map(|s| s.try_into())
                    .collect::<Result<_, _>>()?,
            ),
            Before { slot } => NativeScript::InvalidHereafter(slot),
            After { slot } => NativeScript::InvalidBefore(slot),
        })
    }
}

impl TryFrom<ScriptNative> for pallas_primitives::conway::NativeScript {
    type Error = BlockfrostError;

    fn try_from(script: ScriptNative) -> Result<Self, Self::Error> {
        use pallas_primitives::conway::NativeScript;
        Ok(match script {
            ScriptNative::Any(scripts) => NativeScript::ScriptAny(
                scripts
                    .into_iter()
                    .map(|s| s.try_into())
                    .collect::<Result<_, _>>()?,
            ),
            ScriptNative::All(scripts) => NativeScript::ScriptAll(
                scripts
                    .into_iter()
                    .map(|s| s.try_into())
                    .collect::<Result<_, _>>()?,
            ),
            ScriptNative::ExpiresAt(time) => NativeScript::InvalidHereafter(time),
            ScriptNative::StartsAt(time) => NativeScript::InvalidBefore(time),
            ScriptNative::NOf(h_map) => {
                let (n_str, scripts) = h_map
                    .into_iter()
                    .next()
                    .ok_or_else(|| BlockfrostError::custom_400("NOf: empty map".to_string()))?;
                NativeScript::ScriptNOfK(
                    n_str.parse::<u32>().map_err(|e| {
                        BlockfrostError::custom_400(format!("NOf: invalid n key: {e}"))
                    })?,
                    scripts
                        .into_iter()
                        .map(|s| s.try_into())
                        .collect::<Result<_, _>>()?,
                )
            },
            ScriptNative::Signature(st) => {
                NativeScript::ScriptPubkey(decode_pubkey_hash(&st)?.into())
            },
        })
    }
}

#[derive(Deserialize, Debug)]
pub struct Value {
    pub coins: u64,
    pub assets: Option<HashMap<String, u64>>, // asset name and amount (PositiveCoin in Conway)
}

// This is originally missing PlutusV3 since blockfrost uses Ogmios v5.6 which has slightly different data structure
#[derive(Deserialize, Debug, Clone)]
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

#[derive(Serialize, Deserialize)]
pub struct ExecCost {
    pub memory: u64,
    pub cpu: u64,
}

#[derive(Serialize, Deserialize)]
pub struct ExecCostV5 {
    pub memory: u64,
    pub steps: u64,
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum TxEvalResultV5 {
    Success(TxEvalSuccessV5),
    Failure(TxEvalFailureV5),
}

#[derive(Deserialize)]
pub struct TxEvalSuccessV5 {
    pub tag: RedeemerTag,
    pub index: u32,
    pub units: ExecCostV5,
}

#[derive(Serialize, Deserialize)]
pub struct TxEvalFailureV5 {
    pub validator: TxValidator,
    pub error: serde_json::Value,
}

pub struct TxValidator {
    pub tag: RedeemerTag,
    pub index: u32,
}

// This is also the format ledger responses
#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum TxEvalResultV6 {
    Success(TxEvalSuccessV6),
    Failure(TxEvalFailureV6),
}

#[derive(Serialize)]
pub struct TxValidatorV6 {
    pub purpose: String,
    pub index: u32,
}

#[derive(Serialize, Deserialize)]
pub struct TxEvalSuccessV6 {
    pub validator: TxValidatorV6,
    pub budget: ExecCost,
}

#[derive(Serialize, Deserialize)]
pub struct TxEvalFailureV6 {
    pub validator: TxValidatorV6,
    pub error: serde_json::Value,
}

impl From<TxEvalSuccessV5> for TxEvalResult {
    fn from(value: TxEvalSuccessV5) -> Self {
        TxEvalResult {
            tag: value.tag,
            index: value.index,
            units: value.units.into(),
            logs: Vec::new(),
            success: true,
        }
    }
}

impl From<TxEvalFailureV5> for TxEvalResult {
    fn from(value: TxEvalFailureV5) -> Self {
        TxEvalResult {
            tag: value.validator.tag,
            index: value.validator.index,
            units: ExUnits { mem: 0, steps: 0 },
            logs: vec![value.error.to_string()],
            success: false,
        }
    }
}

impl From<TxEvalResultV5> for TxEvalResult {
    fn from(value: TxEvalResultV5) -> Self {
        match value {
            TxEvalResultV5::Success(success) => TxEvalResult::from(success),
            TxEvalResultV5::Failure(fail) => TxEvalResult::from(fail),
        }
    }
}

impl From<TxEvalResult> for TxEvalSuccessV5 {
    fn from(value: TxEvalResult) -> Self {
        TxEvalSuccessV5 {
            tag: value.tag,
            index: value.index,
            units: value.units.into(),
        }
    }
}

impl From<TxEvalResult> for TxEvalFailureV5 {
    fn from(value: TxEvalResult) -> Self {
        TxEvalFailureV5 {
            error: serde_json::Value::String(value.logs.join(",")),
            validator: TxValidator {
                tag: value.tag,
                index: value.index,
            },
        }
    }
}

impl From<TxEvalResult> for TxEvalResultV5 {
    fn from(value: TxEvalResult) -> Self {
        if value.success {
            TxEvalResultV5::Success(value.into())
        } else {
            TxEvalResultV5::Failure(value.into())
        }
    }
}

impl TryFrom<TxEvalSuccessV6> for TxEvalSuccessV5 {
    type Error = String;
    fn try_from(value: TxEvalSuccessV6) -> Result<Self, Self::Error> {
        let v: TxValidator = value.validator.try_into()?;
        Ok(TxEvalSuccessV5 {
            tag: v.tag,
            index: v.index,
            units: value.budget.into(),
        })
    }
}

impl TryFrom<TxEvalFailureV6> for TxEvalFailureV5 {
    type Error = String;
    fn try_from(value: TxEvalFailureV6) -> Result<Self, Self::Error> {
        Ok(TxEvalFailureV5 {
            validator: value.validator.try_into()?,
            error: value.error,
        })
    }
}

impl TryFrom<TxEvalResultV6> for TxEvalResultV5 {
    type Error = String;
    fn try_from(value: TxEvalResultV6) -> Result<Self, Self::Error> {
        match value {
            TxEvalResultV6::Success(success) => Ok(TxEvalResultV5::Success(success.try_into()?)),
            TxEvalResultV6::Failure(fail) => Ok(TxEvalResultV5::Failure(fail.try_into()?)),
        }
    }
}

pub fn string_to_redeemer_tag(s: &str) -> Option<RedeemerTag> {
    match s {
        "spend" => Some(RedeemerTag::Spend),
        "mint" => Some(RedeemerTag::Mint),
        "cert" => Some(RedeemerTag::Cert),
        "reward" => Some(RedeemerTag::Reward),
        "vote" => Some(RedeemerTag::Vote),
        "propose" => Some(RedeemerTag::Propose),
        _ => None,
    }
}

pub fn redeemer_tag_to_string(tag: RedeemerTag) -> String {
    match tag {
        RedeemerTag::Spend => "spend".to_string(),
        RedeemerTag::Mint => "mint".to_string(),
        RedeemerTag::Cert => "cert".to_string(),
        RedeemerTag::Reward => "reward".to_string(),
        RedeemerTag::Vote => "vote".to_string(),
        RedeemerTag::Propose => "propose".to_string(),
    }
}

use serde::ser::Serializer;
use serde_json::json;

impl Serialize for TxEvalSuccessV5 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(1))?;
        let tag = format!("{}:{}", redeemer_tag_to_string(self.tag), self.index);
        map.serialize_entry(&tag, &self.units)?;
        map.end()
    }
}

impl Serialize for TxValidator {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let str = format!("{}:{}", redeemer_tag_to_string(self.tag), self.index);

        serializer.serialize_str(&str)
    }
}
impl FromStr for TxValidator {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<_> = s.split(':').collect();

        if parts.len() == 2 {
            let tag = string_to_redeemer_tag(parts[0])
                .ok_or_else(|| format!("Invalid tag: {}", parts[0]))?;
            let index: u32 = parts[1]
                .parse()
                .map_err(|_| "Invalid index format".to_string())?;

            Ok(TxValidator { tag, index })
        } else {
            Err(format!("Invalid input format for TxValidator: {s}"))
        }
    }
}

impl<'de> Deserialize<'de> for TxValidator {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Visitor;

        struct TxValidatorVisitor;

        impl Visitor<'_> for TxValidatorVisitor {
            type Value = TxValidator;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a string representing a TxValidator")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                FromStr::from_str(v).map_err(de::Error::custom)
            }
        }

        deserializer.deserialize_str(TxValidatorVisitor)
    }
}

/// Deserializes from ledger string format ("spend:0") and converts to v6 object.
impl<'de> Deserialize<'de> for TxValidatorV6 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let v = TxValidator::from_str(&s).map_err(serde::de::Error::custom)?;
        Ok(TxValidatorV6::from(&v))
    }
}

impl TryFrom<TxValidatorV6> for TxValidator {
    type Error = String;
    fn try_from(v: TxValidatorV6) -> Result<Self, Self::Error> {
        let tag = string_to_redeemer_tag(&v.purpose)
            .ok_or_else(|| format!("unknown redeemer purpose: '{}'", v.purpose))?;
        Ok(TxValidator {
            tag,
            index: v.index,
        })
    }
}

impl From<&TxValidator> for TxValidatorV6 {
    fn from(v: &TxValidator) -> Self {
        TxValidatorV6 {
            purpose: redeemer_tag_to_string(v.tag),
            index: v.index,
        }
    }
}

impl From<ExecCost> for ExUnits {
    fn from(value: ExecCost) -> Self {
        ExUnits {
            mem: value.memory,
            steps: value.cpu,
        }
    }
}

impl From<ExecCostV5> for ExUnits {
    fn from(value: ExecCostV5) -> Self {
        ExUnits {
            mem: value.memory,
            steps: value.steps,
        }
    }
}

impl From<ExUnits> for ExecCostV5 {
    fn from(value: ExUnits) -> Self {
        ExecCostV5 {
            memory: value.mem,
            steps: value.steps,
        }
    }
}

impl From<ExecCost> for ExecCostV5 {
    fn from(value: ExecCost) -> Self {
        ExecCostV5 {
            memory: value.memory,
            steps: value.cpu,
        }
    }
}

impl From<ExUnits> for ExecCost {
    fn from(value: ExUnits) -> Self {
        ExecCost {
            memory: value.mem,
            cpu: value.steps,
        }
    }
}

pub fn convert_eval_report_v5(pallas_report: EvalReport) -> Vec<TxEvalResultV5> {
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
            utxo_set[0].transaction_id(),
            "stringstringstringstringstringstringstringstringstringstringstri"
        );
        assert_eq!(utxo_set[0].output_index(), 4294967295u64);
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
