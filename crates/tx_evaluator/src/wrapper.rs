//! Wraps the evaluation result compatible with the relevant Ogmios version

use serde_json::json;
use testgen::testgen::TestgenResponse;

pub fn wrap_response_v5(resp: TestgenResponse, mirror: serde_json::Value) -> serde_json::Value {
    match resp {
        TestgenResponse::Ok(value) => wrap_success_response_v5(value, mirror),
        // @todo fault format needs to be crafted
        TestgenResponse::Err(err) => json!({
            "type": "jsonwsp/fault",
            "version": "1.0",
            "servicename": "ogmios",
            "fault": {
                "code": "client",
                "string": err,
            },
            "reflection": mirror
        }),
    }
}
pub fn wrap_success_response_v5(
    value: serde_json::Value,
    mirror: serde_json::Value,
) -> serde_json::Value {
    json!({
        "type": "jsonwsp/response",
        "version": "1.0",
        "servicename": "ogmios",
        "methodname": "EvaluateTx",
        "result": value,
        "reflection": mirror,
    })
}

pub fn wrap_response_v6(resp: TestgenResponse, id: serde_json::Value) -> serde_json::Value {
    match resp {
        TestgenResponse::Ok(value) => wrap_success_response_v6(value, id),
        TestgenResponse::Err(err) =>
        // @todo error format needs to be crafted
        {
            json!({
                "jsonrpc": "2.0",
                "method": "evaluateTransaction",
                "error": {
                    "code": 0,
                    "message": err,
                    "data": ""
                },
                "id": id,
            }
            )
        },
    }
}

pub fn wrap_success_response_v6(
    value: serde_json::Value,
    id: serde_json::Value,
) -> serde_json::Value {
    json!({
        "jsonrpc": "2.0",
        "method": "evaluateTransaction",
        "result": convert_ledger_result_to_v6(value),
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
