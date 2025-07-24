mod tests {
    use serde_json::json;
    use tower::ServiceExt;

    use crate::common::initialize_app;

    use axum::{
        body::{Body, to_bytes},
        http::Request,
    };
    use reqwest::Method;

    ///
    /// This test is identical to the Blockfrost test:
    /// https://github.com/blockfrost/blockfrost-tests/blob/dc33312126e9d7c49836d7605ab72224a337bc91/src/fixtures/preview/utils/txs-evaluate-utxos.ts#L5
    /// https://github.com/blockfrost/blockfrost-tests/blob/dc33312126e9d7c49836d7605ab72224a337bc91/src/fixtures/preview/utils/txs-evaluate-utxos.ts#L22
    ///
    #[tokio::test]
    async fn success_cbor_only() {
        // init our app
        let app = initialize_app().await;

        let input = json!({
            "cbor": "84A300818258204E9A66B7E310F004893EEF615E11F8AE6C3328CF2BFDB32F6E40063636D42D7C00018182581D70C40F9129C2684046EB02325B96CA2899A6FA6478C1DDE9B5C53206A51A00D59F800200A10581840000D8799F4D48656C6C6F2C20576F726C6421FF820000F5F6",
        });

        // prepare the request
        let request = Request::builder()
            .method(Method::POST)
            .uri("/utils/tx/evaluate/utxos")
            .header("Content-Type", "application/json")
            .body(Body::from(input.to_string()))
            .unwrap();

        // send the request and get the response
        let response = app
            .oneshot(request)
            .await
            .expect("Request to /utils/tx/evaluate failed");

        assert!(response.status().is_success(), "Response should success");

        // Convert the response body to bytes
        let body_bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("Failed to read response body");

        // Convert the bytes to a string and print
        let body_str = String::from_utf8_lossy(&body_bytes);

        assert_eq!(
            body_str,
            r#"[{"budget":{"cpu":3776833,"memory":15694},"validator":"spend:0"}]"#
        );
    }

    /// This test is identical to the Blockfrost test:
    /// https://github.com/blockfrost/blockfrost-tests/blob/7a847bc41b8153844a2643d817559367cc4ffd4d/src/fixtures/preview/utils/txs-evaluate.ts#L5
    ///
    #[tokio::test]
    #[ignore = "not implemented yet"]
    async fn fail_missing_input() {
        // init our app
        let app = initialize_app().await;

        let input = json!({
            "cbor": "".to_string() +
            "84A60082825820000000000000000000000000000000000000000000000000" +
        "0000000000000000182A8258207D67D80BC5B3BADCAF02375E428A39AEA398" +
        "DD0438F26899A1B265C6AC87EB6B000D81825820DB7DBF9EAA6094982ED4B9" +
        "B735CE275345F348194A7E8E9200FEC7D1CAD008EB010181825839004A294F" +
        "1EF53B30CDBF7CAF17798422A90227224F9FBF037FCF6C47A5BC2EC1952D11" +
        "89886FE018214EED45F83AB04171C41F373D530CA7A61A3BB94E8002000E80" +
        "0B58206DF8859EC92C3FF6BC0E2964793789E44E4C5ABBCC9FF6F2387B94F4" +
        "C2020E6EA303814E4D01000033222220051200120011048180058184000018" +
        "2A820000F5F6",
        "additionalUtxoSet": [
                  [
                    {"index":42,"txId":"0000000000000000000000000000000000000000000000000000000000000000"},
                    {"address":"addr_test1wpnlxv2xv9a9ucvnvzqakwepzl9ltx7jzgm53av2e9ncv4sysemm8","value":{"coins":200000}, "datumHash": "45b0cfc220ceec5b7c1c62c4d4193d38e4eba48e8815729ce75f9c0ab0e4c1c0"}
                  ],
            ]
        });

        // prepare the request
        let request = Request::builder()
            .method(Method::POST)
            .uri("/utils/tx/evaluate/utxos")
            .header("Content-Type", "application/json")
            .body(Body::from(input.to_string()))
            .unwrap();

        // send the request and get the response
        let response = app
            .oneshot(request)
            .await
            .expect("Request to /utils/tx/evaluate failed");

        assert!(!response.status().is_success(), "Response should fail");

        // Convert the response body to bytes
        let body_bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("Failed to read response body");

        // Convert the bytes to a string and print
        let body_str = String::from_utf8_lossy(&body_bytes);

        assert_eq!(
            body_str,
            "{\"error\":\"Bad Request\",\"message\":\"Error evaluating transaction: resolved Input not found\",\"status_code\":400}"
        );
    }
}
