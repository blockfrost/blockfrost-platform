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
        TestgenResponse::Err(_err) => todo!("err not yet here"),
    }
}

pub fn wrap_response_v6(resp: TestgenResponse, id: serde_json::Value) -> serde_json::Value {
    match resp {
        TestgenResponse::Ok(value) => {
            json!({
                "jsonrpc": "2.0",
                "result": value,
                "id": id,
            }
            )
        },
        TestgenResponse::Err(_err) => todo!("err not yet here"),
    }
}
