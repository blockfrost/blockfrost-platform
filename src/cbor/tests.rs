use super::fallback_decoder::FallbackDecoder;
use serde::Deserialize;
use std::process::Command;

mod random;
mod specific;

#[derive(Deserialize, Debug)]
#[serde(rename_all(deserialize = "camelCase"))]
pub struct CborTestCases {
    pub seed: u64,
    pub test_cases: Vec<CborTestCase>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all(deserialize = "camelCase"))]
pub struct CborTestCase {
    pub cbor: String,
    pub haskell_repr: String,
    pub json: serde_json::Value,
}

#[derive(Debug)]
#[allow(non_camel_case_types, dead_code)]
pub enum CaseType {
    ApplyTxErr_Byron,
    ApplyTxErr_Shelley,
    ApplyTxErr_Allegra,
    ApplyTxErr_Mary,
    ApplyTxErr_Alonzo,
    ApplyTxErr_Babbage,
    ApplyTxErr_Conway,
    GHCInteger,
    DataText,
    ExampleADT,
}

pub fn generate_cases(
    case_type: CaseType,
    num_cases: u32,
    generator_size: u16,
    seed: Option<u64>,
) -> Result<CborTestCases, String> {
    let child_exe = super::fallback_decoder::FallbackDecoder::find_testgen_hs(&[]).unwrap();

    let output = Command::new(&child_exe)
        .arg("generate")
        .args(["--number", &num_cases.to_string()])
        .args(["--generator-size", &generator_size.to_string()])
        .args(seed.map_or(vec![], |seed| vec!["--seed".to_string(), seed.to_string()]))
        .arg(format!("{:?}", case_type))
        .output()
        .map_err(|err| format!("{}: failed to execute: {}", child_exe, err))?;

    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    if !stderr.is_empty() {
        Err(format!("{}: stderr non-empty: {}", child_exe, stderr))?
    }

    if !output.status.success() {
        Err(format!(
            "{}: exited with code: {}",
            child_exe, output.status
        ))?
    }

    let stdout = String::from_utf8(output.stdout)
        .map_err(|err| format!("{}: invalid UTF-8 in stdout: {}", child_exe, err))?;

    serde_json::from_str(&stdout)
        .map_err(|err| format!("{}: JSON deserialization failed: {}", child_exe, err))
}

macro_rules! assert_json_eq {
    ($left:expr, $right:expr) => {
        if $left != $right {
            let left_pretty = serde_json::to_string_pretty(&$left).unwrap();
            let right_pretty = serde_json::to_string_pretty(&$right).unwrap();
            panic!(
                concat!(
                    "assertion `left == right` failed\n",
                    "  left:\n    {}\n  right:\n    {}",
                ),
                left_pretty.replace("\n", "\n    "),
                right_pretty.replace("\n", "\n    "),
            );
        }
    };
}

pub(crate) use assert_json_eq; // export it

/// This function takes a CBOR-encoded `ApplyTxErr`, and verifies our
/// deserializer against the Haskell one. Use it for specific cases.
async fn verify_one(cbor: &str) {
    use crate::node::connection::NodeClient;

    let cbor = hex::decode(cbor).unwrap();
    let reference_json = FallbackDecoder::instance().decode(&cbor).await.unwrap();

    let our_decoding = NodeClient::try_decode_error(&cbor).unwrap_or_else(|err| panic!(
        "Rust deserializer failed to decode:\n  CBOR:\n    {}\n  Error:\n    {}\n  Haskell:\n    {}",
        hex::encode(cbor),
        format!("{:?}", err).replace("\n", "\n    "),
        serde_json::to_string_pretty(&reference_json).unwrap().replace("\n", "\n    ")
    ));

    let our_json = serde_json::to_value(our_decoding).unwrap();
    assert_json_eq!(reference_json, our_json)
}
