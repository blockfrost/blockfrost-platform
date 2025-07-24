mod tests {
    use serde_json::json;
    use tower::ServiceExt;

    use crate::common::initialize_app;

    use axum::{
        body::{Body, to_bytes},
        http::Request,
    };
    use reqwest::Method;

    #[tokio::test]
    #[ignore]
    async fn success() {
        // init our app
        let app = initialize_app().await;

        let input = json!({
            "cbor": "84a800d9010282825820b0a649f2b1fa7d0553d7eb3815fe1d36e893f7a18322be661991be7777f104ab01825820b0a649f2b1fa7d0553d7eb3815fe1d36e893f7a18322be661991be7777f104ab000181825839005c5c318d01f729e205c95eb1b02d623dd10e78ea58f72d0c13f892b2e8904edc699e2f0ce7b72be7cec991df651a222e2ae9244eb5975cba1a9e38e8b8021a0002c0b30b58201124502b69ef6917c02db98f8026871e95a70fcc864eb8146bfd44237612b4670dd9010281825820b0a649f2b1fa7d0553d7eb3815fe1d36e893f7a18322be661991be7777f104ab0110825839005c5c318d01f729e205c95eb1b02d623dd10e78ea58f72d0c13f892b2e8904edc699e2f0ce7b72be7cec991df651a222e2ae9244eb5975cba1a9841a75e111a0004210d12d90102818258205a3b16ae983c3623b6f516a1a1bcafc124ddd7bebf8406f541c27f24a97d8d8c00a200d90102818258202a60dcffe8ba15307556dbf8d7df142cb9eb15d601251d400d523689d575b8385840843e477f1885303639acf78b1faf47e2c6c81930ef7570c5e843a038ec8042cd3784d89ababc1d803ecd25972ebd3a50da71dd90a9182dbfdbfd8b94a002560305a182000082d8799f4568656c6c6fff82194d101a005ee1c6f5f6",
            "additionalUtxoSet": [
                  [
                    {"index":0,"txId":"b0a649f2b1fa7d0553d7eb3815fe1d36e893f7a18322be661991be7777f104ab"},
                    {"address":"70faae60072c45d121b6e58ae35c624693ee3dad9ea8ed765eb6f76f9f","value":{"coins":100270605}}
                  ],
                  [
                    {"index":0,"txId":"5a3b16ae983c3623b6f516a1a1bcafc124ddd7bebf8406f541c27f24a97d8d8c"},
                    {"address":"70faae60072c45d121b6e58ae35c624693ee3dad9ea8ed765eb6f76f9f","script":{"plutus:v3":"58a701010032323232323225333002323232323253330073370e900118041baa0011323322533300a3370e900018059baa00513232533300f30110021533300c3370e900018069baa00313371e6eb8c040c038dd50039bae3010300e37546020601c6ea800c5858dd7180780098061baa00516300c001300c300d001300937540022c6014601600660120046010004601000260086ea8004526136565734aae7555cf2ab9f5742ae89"},"value":{"coins":1624870}}
                  ],
                  [
                    {"index":1,"txId":"b0a649f2b1fa7d0553d7eb3815fe1d36e893f7a18322be661991be7777f104ab"},
                    {"address":"005c5c318d01f729e205c95eb1b02d623dd10e78ea58f72d0c13f892b2e8904edc699e2f0ce7b72be7cec991df651a222e2ae9244eb5975cba","value":{"coins":2554439518u64}}
                    ]

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

        assert!(
            response.status().is_success(),
            "Response was not successful"
        );

        // Convert the response body to bytes
        let body_bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("Failed to read response body");

        // Convert the bytes to a string and print
        let body_str = String::from_utf8_lossy(&body_bytes);

        assert_eq!(body_str, "[{\"spend:0\":{\"memory\":958,\"cpu\":2348473}}]");
    }
}
