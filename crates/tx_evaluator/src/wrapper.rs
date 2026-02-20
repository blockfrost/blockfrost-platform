//! Wraps the evaluation result compatible with the relevant Ogmios version

use bf_testgen::testgen::TestgenResponse;
use serde_json::{Map, json};

use crate::{
    model::api::{OgmiosError, TxEvalResultV5, TxEvalResultV6},
    ogmios5_response::to_ogmios_error_v5,
};

/**
 * V5 responses does generate three kind of json:
 * - successfull eval; top-level 'result' field with exec units
 * - failed eval: top-level 'result' field with error json details
 * - faulty input: top-level 'fault' field with error string
 */
pub fn wrap_response_v5(resp: TestgenResponse, mirror: serde_json::Value) -> serde_json::Value {
    //println!("wrap_response_v5: {:?}", resp);
    match resp {
        TestgenResponse::Ok(value) => {
            // The external evaluator returns a v6-like structure, but we need to convert it to v5.
            let v6_results: Vec<TxEvalResultV6> = serde_json::from_value(value).unwrap();
            let v5: Vec<TxEvalResultV5> = v6_results.into_iter().map(|r| r.into()).collect();
            wrap_success_response_v5(v5, mirror)
        },
        TestgenResponse::Err(err) => {
            let err = error_value_to_string(err);
            to_ogmios_error_v5(generate_error_response_v6(err), mirror)
        },
    }
}

pub fn wrap_success_response_v5(
    response: Vec<TxEvalResultV5>,
    mirror: serde_json::Value,
) -> serde_json::Value {
    // flatten objects
    let mut result_map = Map::with_capacity(response.len());
    for r in response {
        serde_json::to_value(r)
            .unwrap()
            .as_object()
            .unwrap()
            .iter()
            .for_each(|(key, val)| {
                result_map.insert(key.to_string(), val.clone());
            });
    }

    json!({
        "type": "jsonwsp/response",
        "version": "1.0",
        "servicename": "ogmios",
        "methodname": "EvaluateTx",
        "result": result_map,
        "reflection": mirror,
    })
}

pub fn wrap_response_v6(resp: TestgenResponse, id: serde_json::Value) -> serde_json::Value {
    match resp {
        TestgenResponse::Ok(value) => {
            println!("value6 {:?}", value);
            let decoded: Vec<TxEvalResultV6> = serde_json::from_value(value).unwrap();

            if is_success_v6(&decoded) {
                wrap_success_response_v6(decoded, id)
            } else {
                {
                    json!({
                        "jsonrpc": "2.0",
                        "method": "evaluateTransaction",
                        "error": "hello error",
                        "id": id,
                    }
                    )
                }
            }
        },
        TestgenResponse::Err(err) =>
        // @todo error format needs to be crafted
        {
            let err = error_value_to_string(err);
            json!({
                "jsonrpc": "2.0",
                "method": "evaluateTransaction",
                "error": generate_error_response_v6(err),
                "id": id,
            }
            )
        },
    }
}

pub fn is_success_v6(results: &[TxEvalResultV6]) -> bool {
    if let Some(first_result) = results.first() {
        matches!(first_result, TxEvalResultV6::SUCCESS(_))
    } else {
        false
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
    id: serde_json::Value,
) -> serde_json::Value {
    json!({
        "jsonrpc": "2.0",
        "method": "evaluateTransaction",
        "result": serde_json::to_value(value).unwrap(),
        "id": id,
    }
    )
}

// This function converts the ledger result(coming from the external binary) to v6.
pub fn convert_ledger_result_to_v6(result: serde_json::Value) -> serde_json::Value {
    // The `result` from the external evaluator is in v5 format:
    // an object like `{"spend:0": {"cpu": ..., "memory": ...}}`
    // We need to convert it to v6 format:
    // an array like `[{"validator": "spend:0", "budget": {"cpu": ..., "memory": ...}}]`
    let result_obj = match result.as_object() {
        Some(obj) => obj,
        // If it's not an object, we can't convert it, so return it as is.
        None => return result,
    };

    let v6_result_arr: Vec<serde_json::Value> = result_obj
        .iter()
        .map(|(validator, budget)| json!({ "validator": validator, "budget": budget }))
        .collect();

    serde_json::Value::Array(v6_result_arr)
}
