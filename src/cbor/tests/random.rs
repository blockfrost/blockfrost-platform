use super::*;

#[test]
#[allow(non_snake_case)]
#[cfg(not(feature = "tarpaulin"))]
fn proptest_ApplyTxErr_Conway_size_001() {
    proptest_with_params(CaseType::ApplyTxErr_Conway, 100000, 1, None)
}

#[test]
#[allow(non_snake_case)]
#[cfg(not(feature = "tarpaulin"))]
fn proptest_ApplyTxErr_Conway_size_002() {
    proptest_with_params(CaseType::ApplyTxErr_Conway, 75000, 2, None)
}

#[test]
#[allow(non_snake_case)]
#[cfg(not(feature = "tarpaulin"))]
fn proptest_ApplyTxErr_Conway_size_003() {
    proptest_with_params(CaseType::ApplyTxErr_Conway, 50000, 3, None)
}

#[test]
#[allow(non_snake_case)]
#[cfg(not(feature = "tarpaulin"))]
fn proptest_ApplyTxErr_Conway_size_004() {
    proptest_with_params(CaseType::ApplyTxErr_Conway, 20000, 4, None)
}

#[test]
#[allow(non_snake_case)]
#[cfg(not(feature = "tarpaulin"))]
fn proptest_ApplyTxErr_Conway_size_005() {
    proptest_with_params(CaseType::ApplyTxErr_Conway, 10000, 5, None)
}

#[test]
#[allow(non_snake_case)]
#[cfg(not(feature = "tarpaulin"))]
fn proptest_ApplyTxErr_Conway_size_006() {
    proptest_with_params(CaseType::ApplyTxErr_Conway, 10000, 6, None)
}

#[test]
#[allow(non_snake_case)]
#[cfg(not(feature = "tarpaulin"))]
fn proptest_ApplyTxErr_Conway_size_007() {
    proptest_with_params(CaseType::ApplyTxErr_Conway, 10000, 7, None)
}

#[test]
#[allow(non_snake_case)]
#[cfg(not(feature = "tarpaulin"))]
fn proptest_ApplyTxErr_Conway_size_008() {
    proptest_with_params(CaseType::ApplyTxErr_Conway, 10000, 8, None)
}

#[test]
#[allow(non_snake_case)]
#[cfg(not(feature = "tarpaulin"))]
fn proptest_ApplyTxErr_Conway_size_009() {
    proptest_with_params(CaseType::ApplyTxErr_Conway, 10000, 9, None)
}

#[test]
#[allow(non_snake_case)]
#[cfg(not(feature = "tarpaulin"))]
fn proptest_ApplyTxErr_Conway_size_010() {
    proptest_with_params(CaseType::ApplyTxErr_Conway, 10000, 10, None)
}

#[test]
#[allow(non_snake_case)]
#[cfg(not(feature = "tarpaulin"))]
fn proptest_ApplyTxErr_Conway_size_015() {
    proptest_with_params(CaseType::ApplyTxErr_Conway, 6000, 15, None)
}

#[test]
#[allow(non_snake_case)]
#[cfg(not(feature = "tarpaulin"))]
fn proptest_ApplyTxErr_Conway_size_020() {
    proptest_with_params(CaseType::ApplyTxErr_Conway, 4000, 20, None)
}

#[test]
#[allow(non_snake_case)]
#[cfg(not(feature = "tarpaulin"))]
fn proptest_ApplyTxErr_Conway_size_025() {
    proptest_with_params(CaseType::ApplyTxErr_Conway, 3000, 25, None)
}

#[test]
#[allow(non_snake_case)]
#[cfg(not(feature = "tarpaulin"))]
fn proptest_ApplyTxErr_Conway_size_030() {
    proptest_with_params(CaseType::ApplyTxErr_Conway, 2500, 30, None)
}

#[test]
#[allow(non_snake_case)]
#[cfg(not(feature = "tarpaulin"))]
fn proptest_ApplyTxErr_Conway_size_035() {
    proptest_with_params(CaseType::ApplyTxErr_Conway, 2000, 35, None)
}

#[test]
#[allow(non_snake_case)]
#[cfg(not(feature = "tarpaulin"))]
fn proptest_ApplyTxErr_Conway_size_040() {
    proptest_with_params(CaseType::ApplyTxErr_Conway, 1000, 40, None)
}

#[test]
#[allow(non_snake_case)]
#[cfg(not(feature = "tarpaulin"))]
fn proptest_ApplyTxErr_Conway_size_045() {
    proptest_with_params(CaseType::ApplyTxErr_Conway, 1000, 45, None)
}

#[test]
#[allow(non_snake_case)]
#[cfg(not(feature = "tarpaulin"))]
fn proptest_ApplyTxErr_Conway_size_050() {
    proptest_with_params(CaseType::ApplyTxErr_Conway, 1000, 50, None)
}

#[test]
#[allow(non_snake_case)]
#[cfg(not(feature = "tarpaulin"))]
fn proptest_ApplyTxErr_Conway_size_075() {
    proptest_with_params(CaseType::ApplyTxErr_Conway, 500, 75, None)
}

#[test]
#[allow(non_snake_case)]
#[cfg(not(feature = "tarpaulin"))]
fn proptest_ApplyTxErr_Conway_size_100() {
    proptest_with_params(CaseType::ApplyTxErr_Conway, 500, 100, None)
}

#[test]
#[allow(non_snake_case)]
#[cfg(not(feature = "tarpaulin"))]
fn proptest_ApplyTxErr_Conway_size_125() {
    proptest_with_params(CaseType::ApplyTxErr_Conway, 100, 125, None)
}

#[test]
#[allow(non_snake_case)]
#[cfg(not(feature = "tarpaulin"))]
fn proptest_ApplyTxErr_Conway_size_150() {
    proptest_with_params(CaseType::ApplyTxErr_Conway, 20, 150, None)
}

#[test]
#[allow(non_snake_case)]
#[cfg(not(feature = "tarpaulin"))]
fn proptest_ApplyTxErr_Conway_size_200() {
    proptest_with_params(CaseType::ApplyTxErr_Conway, 10, 200, None)
}

#[test]
#[allow(non_snake_case)]
#[cfg(not(feature = "tarpaulin"))]
fn proptest_ApplyTxErr_Conway_size_300() {
    proptest_with_params(CaseType::ApplyTxErr_Conway, 3, 300, None)
}

/// Tests the native Rust deserializer with the given params.
///
/// To generate data for [the
/// spreadsheet](https://docs.google.com/spreadsheets/d/1ekbk9bgAAZUX9VevM9U5zdWpT8phHMrhvepyMvL3CAo),
/// run something like:
///
/// ```text
/// ❯ cargo test proptest_ApplyTxErr_Conway 2>&1 \
///     | grep -E '^For size ([0-9]+): ([0-9]+) out of ([0-9]+) .*$' \
///     | sed  -r 's/^For size ([0-9]+): ([0-9]+) out of ([0-9]+) .*$/\1\t\2\t\3/g' \
///     | sort -n
/// ```
fn proptest_with_params(
    case_type: CaseType,
    num_cases: u32,
    generator_size: u16,
    seed: Option<u64>,
) {
    check_generated_cases(case_type, num_cases, generator_size, 5, seed, |case| {
        let cbor = case.cbor.clone();

        let test_one = move || {
            let cbor = hex::decode(case.cbor).map_err(|e| e.to_string())?;
            let our_json = serialize_error(decode_error(&cbor));

            if our_json == case.json {
                Ok(())
            } else {
                Err("".to_string())
            }
        };

        if test_one().is_err() {
            Err(cbor)
        } else {
            Ok(())
        }
    })
}

fn decode_error(bytes: &[u8]) -> TxValidationError {
    use pallas_codec::minicbor;

    let mut decoder = minicbor::Decoder::new(bytes);
    decoder.decode().unwrap()
}
