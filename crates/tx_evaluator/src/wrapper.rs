//! Wraps the evaluation result compatible with the relevant Ogmios version

use bf_testgen::testgen::TestgenResponse;
use serde_json::{Map, json};

use crate::{
    helper::generate_reflection_v5,
    helper::generate_reflection_v6,
    model::api::{OgmiosError, TxEvalResultV5, TxEvalResultV6},
    ogmios5_response::{process_error_array, to_ogmios_error_v5},
};

use crate::external::EvalOutput;

pub fn wrap_eval_output_v5(output: EvalOutput) -> serde_json::Value {
    match output {
        EvalOutput::Testgen(resp) => wrap_response_v5(resp),
        EvalOutput::Error(oe) => to_ogmios_error_v5(&oe, &generate_reflection_v5()),
    }
}

pub fn wrap_eval_output_v6(output: EvalOutput) -> serde_json::Value {
    match output {
        EvalOutput::Testgen(resp) => wrap_response_v6(resp),
        EvalOutput::Error(oe) => wrap_ogmios_error_v6(&oe, &generate_reflection_v6()),
    }
}

/// V5 responses does generate three kind of json:
/// - successful eval; top-level 'result' field with exec units
/// - failed eval: top-level 'result' field with error json details
/// - faulty input: top-level 'fault' field with error string
fn wrap_response_v5(resp: TestgenResponse) -> serde_json::Value {
    let reflection = generate_reflection_v5();
    match resp {
        TestgenResponse::Ok(value) => {
            // The external evaluator returns a v6-like structure, but we need to convert it to v5.
            let v6_results: Vec<TxEvalResultV6> = match serde_json::from_value(value) {
                Ok(r) => r,
                Err(e) => {
                    tracing::error!("ExternalEvaluator: failed to parse evaluation result: {e}");
                    let oe = generate_error_response_v6(
                        "Something went wrong. Failed to parse evaluator result.".to_string(),
                    );
                    return to_ogmios_error_v5(&oe, &reflection);
                },
            };

            if is_success_v6(&v6_results) {
                let v5: Result<Vec<TxEvalResultV5>, String> =
                    v6_results.into_iter().map(|r| r.try_into()).collect();
                let v5 = match v5 {
                    Ok(v) => v,
                    Err(e) => {
                        tracing::error!("ExternalEvaluator: v6→v5 conversion failed: {e}");
                        let oe = generate_error_response_v6(
                            "Something went wrong. Failed to convert evaluation result."
                                .to_string(),
                        );
                        return to_ogmios_error_v5(&oe, &reflection);
                    },
                };
                wrap_success_response_v5(v5, &reflection)
            } else {
                let failure_values: Vec<serde_json::Value> = v6_results
                    .into_iter()
                    .filter_map(|r| match r {
                        TxEvalResultV6::Failure(f) => serde_json::to_value(f).ok(),
                        _ => None,
                    })
                    .collect();
                wrap_error_response_processed_v5(&failure_values, &reflection)
            }
        },
        TestgenResponse::Err(err) => {
            let err = error_value_to_string(err);
            to_ogmios_error_v5(&generate_error_response_v6(err), &reflection)
        },
    }
}

pub fn wrap_error_response_processed_v5(
    failures: &[serde_json::Value],
    reflection: &serde_json::Value,
) -> serde_json::Value {
    let result = process_error_array(failures);
    wrap_error_response_v5(&result, reflection)
}

pub fn wrap_error_response_v5(
    result: &serde_json::Value,
    reflection: &serde_json::Value,
) -> serde_json::Value {
    json!({
        "type": "jsonwsp/response",
        "version": "1.0",
        "servicename": "ogmios",
        "methodname": "EvaluateTx",
        "result": result,
        "reflection": reflection,
    })
}

pub fn wrap_as_incompatible_era_v5(era: String) -> serde_json::Value {
    wrap_error_response_v5(
        &json!( { "EvaluationFailure": { "IncompatibleEra": era }}  ),
        &generate_reflection_v5(),
    )
}

pub fn wrap_success_response_v5(
    response: Vec<TxEvalResultV5>,
    reflection: &serde_json::Value,
) -> serde_json::Value {
    // flatten objects
    let mut result_map = Map::with_capacity(response.len());
    for r in response {
        let value = match serde_json::to_value(r) {
            Ok(v) => v,
            Err(e) => {
                tracing::error!("TxEvalResultV5: serialization failed: {e}");
                let oe = generate_error_response_v6(
                    "Something went wrong. Failed to serialize evaluation result.".to_string(),
                );
                return to_ogmios_error_v5(&oe, reflection);
            },
        };
        let Some(obj) = value.as_object() else {
            tracing::error!("TxEvalResultV5: expected object, got {}", value);
            let oe = generate_error_response_v6(
                "Something went wrong. Unexpected evaluation result format.".to_string(),
            );
            return to_ogmios_error_v5(&oe, reflection);
        };
        for (key, val) in obj {
            result_map.insert(key.to_string(), val.clone());
        }
    }

    json!({
        "type": "jsonwsp/response",
        "version": "1.0",
        "servicename": "ogmios",
        "methodname": "EvaluateTx",
        "result": { "EvaluationResult": result_map },
        "reflection": reflection,
    })
}

fn wrap_response_v6(resp: TestgenResponse) -> serde_json::Value {
    let id = generate_reflection_v6();
    match resp {
        TestgenResponse::Ok(value) => {
            let decoded: Vec<TxEvalResultV6> = match serde_json::from_value(value) {
                Ok(r) => r,
                Err(e) => {
                    tracing::error!("ExternalEvaluator: failed to parse evaluation result: {e}");
                    let oe = generate_error_response_v6(
                        "Something went wrong. Failed to parse evaluator result.".to_string(),
                    );
                    return wrap_ogmios_error_v6(&oe, &id);
                },
            };

            if is_success_v6(&decoded) {
                wrap_success_response_v6(decoded, &id)
            } else {
                let failures: Vec<_> = decoded
                    .into_iter()
                    .filter_map(|r| {
                        if let TxEvalResultV6::Failure(f) = r {
                            Some(f)
                        } else {
                            None
                        }
                    })
                    .collect();
                wrap_error_response_v6(&json!(failures), &id)
            }
        },

        TestgenResponse::Err(err) => {
            let err = generate_error_response_v6(error_value_to_string(err));
            wrap_ogmios_error_v6(&err, &id)
        },
    }
}

fn wrap_error_response_v6(err: &serde_json::Value, id: &serde_json::Value) -> serde_json::Value {
    json!({
        "jsonrpc": "2.0",
        "method": "evaluateTransaction",
        "error": err,
        "id": id,
    })
}

pub fn wrap_as_incompatible_era_v6(era: String) -> serde_json::Value {
    wrap_error_response_v6(
        &json!(OgmiosError::incompatible_era(era)),
        &generate_reflection_v6(),
    )
}

pub fn wrap_ogmios_error_v6(err: &OgmiosError, id: &serde_json::Value) -> serde_json::Value {
    wrap_error_response_v6(&json!(err), id)
}

/// Checks whether the evaluation results represent success by inspecting only the first element.
/// The evaluator always returns a homogeneous list — either all successes or all failures —
/// so the first element is sufficient to determine the outcome.
pub fn is_success_v6(results: &[TxEvalResultV6]) -> bool {
    if let Some(first_result) = results.first() {
        matches!(first_result, TxEvalResultV6::Success(_))
    } else {
        true
    }
}

fn error_value_to_string(err: serde_json::Value) -> String {
    match err {
        serde_json::Value::String(value) => value,
        other => other.to_string(),
    }
}

pub fn generate_error_response_v6(err: String) -> OgmiosError {
    OgmiosError::deserialization_error(err)
}

pub fn wrap_success_response_v6(
    value: Vec<TxEvalResultV6>,
    id: &serde_json::Value,
) -> serde_json::Value {
    json!({
        "jsonrpc": "2.0",
        "method": "evaluateTransaction",
        "result": value,
        "id": id,
    }
    )
}
