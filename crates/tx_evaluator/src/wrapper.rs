//! Wraps the evaluation result compatible with the relevant Ogmios version

use serde_json::json;
use testgen::testgen::TestgenResponse;

pub fn wrap_response_v5(resp: TestgenResponse, mirror: serde_json::Value) -> serde_json::Value {
    match resp {
        TestgenResponse::Ok(value) => json!({
            "type": "jsonwsp/response",
            "version": "1.0",
            "servicename": "ogmios",
            "methodname": "EvaluateTx",
            "result": value,
            "reflection": mirror,
        }),
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

pub fn wrap_response_v6(resp: TestgenResponse, id: serde_json::Value) -> serde_json::Value {
    match resp {
        TestgenResponse::Ok(value) => {
            json!({
                "jsonrpc": "2.0",
                "method": "evaluateTransaction",
                "result": value,
                "id": id,
            }
            )
        },
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
