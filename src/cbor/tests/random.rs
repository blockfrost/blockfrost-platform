use super::*;

#[test]
#[allow(non_snake_case)]
fn proptest_ApplyTxErr_Conway_1000_size_01() {
    proptest_with_params(CaseType::ApplyTxErr_Conway, 1000000, 1, None)
}

#[test]
#[allow(non_snake_case)]
fn proptest_ApplyTxErr_Conway_1000_size_02() {
    proptest_with_params(CaseType::ApplyTxErr_Conway, 100000, 2, None)
}

#[test]
#[allow(non_snake_case)]
fn proptest_ApplyTxErr_Conway_1000_size_03() {
    proptest_with_params(CaseType::ApplyTxErr_Conway, 100000, 3, None)
}

#[test]
#[allow(non_snake_case)]
fn proptest_ApplyTxErr_Conway_1000_size_04() {
    proptest_with_params(CaseType::ApplyTxErr_Conway, 10000, 4, None)
}

#[test]
#[allow(non_snake_case)]
fn proptest_ApplyTxErr_Conway_1000_size_05() {
    proptest_with_params(CaseType::ApplyTxErr_Conway, 10000, 5, None)
}

#[test]
#[allow(non_snake_case)]
fn proptest_ApplyTxErr_Conway_1000_size_06() {
    proptest_with_params(CaseType::ApplyTxErr_Conway, 10000, 6, None)
}

#[test]
#[allow(non_snake_case)]
fn proptest_ApplyTxErr_Conway_1000_size_07() {
    proptest_with_params(CaseType::ApplyTxErr_Conway, 10000, 7, None)
}

#[test]
#[allow(non_snake_case)]
fn proptest_ApplyTxErr_Conway_1000_size_08() {
    proptest_with_params(CaseType::ApplyTxErr_Conway, 10000, 8, None)
}

#[test]
#[allow(non_snake_case)]
fn proptest_ApplyTxErr_Conway_1000_size_09() {
    proptest_with_params(CaseType::ApplyTxErr_Conway, 10000, 9, None)
}

#[test]
#[allow(non_snake_case)]
fn proptest_ApplyTxErr_Conway_1000_size_10() {
    proptest_with_params(CaseType::ApplyTxErr_Conway, 10000, 10, None)
}

#[test]
#[allow(non_snake_case)]
fn proptest_ApplyTxErr_Conway_1000_size_12() {
    proptest_with_params(CaseType::ApplyTxErr_Conway, 10000, 12, None)
}

#[test]
#[allow(non_snake_case)]
fn proptest_ApplyTxErr_Conway_1000_size_15() {
    proptest_with_params(CaseType::ApplyTxErr_Conway, 10000, 15, None)
}

#[test]
#[allow(non_snake_case)]
fn proptest_ApplyTxErr_Conway_1000_size_16() {
    proptest_with_params(CaseType::ApplyTxErr_Conway, 10000, 16, None)
}

#[test]
#[allow(non_snake_case)]
fn proptest_ApplyTxErr_Conway_1000_size_17() {
    proptest_with_params(CaseType::ApplyTxErr_Conway, 10000, 17, None)
}

#[test]
#[allow(non_snake_case)]
fn proptest_ApplyTxErr_Conway_1000_size_18() {
    proptest_with_params(CaseType::ApplyTxErr_Conway, 10000, 18, None)
}

#[test]
#[allow(non_snake_case)]
fn proptest_ApplyTxErr_Conway_1000_size_19() {
    proptest_with_params(CaseType::ApplyTxErr_Conway, 10000, 19, None)
}

#[test]
#[allow(non_snake_case)]
fn proptest_ApplyTxErr_Conway_1000_size_20() {
    proptest_with_params(CaseType::ApplyTxErr_Conway, 10000, 20, None)
}

#[test]
#[allow(non_snake_case)]
fn proptest_ApplyTxErr_Conway_1000_size_25() {
    proptest_with_params(CaseType::ApplyTxErr_Conway, 10000, 25, None)
}

#[test]
#[allow(non_snake_case)]
fn proptest_ApplyTxErr_Conway_1000_size_30() {
    proptest_with_params(CaseType::ApplyTxErr_Conway, 10000, 30, None)
}

#[test]
#[allow(non_snake_case)]
fn proptest_ApplyTxErr_Conway_1000_size_35() {
    proptest_with_params(CaseType::ApplyTxErr_Conway, 10000, 35, None)
}

#[test]
#[allow(non_snake_case)]
fn proptest_ApplyTxErr_Conway_1000_size_40() {
    proptest_with_params(CaseType::ApplyTxErr_Conway, 10000, 40, None)
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
    let cases = generate_cases(case_type, num_cases, generator_size, seed).unwrap();

    let mut failing_cbor: Vec<String> = vec![];
    let num_all = cases.test_cases.len();

    for case in cases.test_cases {
        use crate::node::connection::NodeClient;

        let cbor = case.cbor.clone();

        let test_one = move || {
            let cbor = hex::decode(case.cbor).map_err(|e| e.to_string())?;
            let our_json = serde_json::to_value(
                NodeClient::try_decode_error(&cbor).map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;
            if our_json == case.json {
                Ok(())
            } else {
                Err("".to_string())
            }
        };

        if test_one().is_err() {
            failing_cbor.push(cbor);
        }
    }

    if !failing_cbor.is_empty() {
        let failed = failing_cbor.len();
        let percent = failed as f64 / num_all as f64 * 100.0;
        let mut details = "".to_string();

        // How many failing examples to show verbatim in test output:
        let show_max = 5;

        if show_max > 0 {
            details.push_str(&format!(
                " Failing CBORs{}:",
                if failed <= show_max {
                    "".to_string()
                } else {
                    format!(" (first {})", show_max)
                }
            ));

            failing_cbor.sort_by_key(|cbor| cbor.len());
            for cbor in failing_cbor.iter().take(show_max) {
                details.push_str(&format!("\n- {}", cbor));
            }
        }

        panic!(
            "For size {}: {} out of {} ({:.2}%) failed.{}",
            generator_size, failed, num_all, percent, details
        )
    }
}
