#[cfg(test)]
mod tests {
    use std::sync::Once;

    use axum::body::{to_bytes, Body};
    use axum::http::Request;
    use blockfrost_platform::{
        cli::{Config, LogLevel, Mode, Network},
        server::build,
    };
    use lazy_static::lazy_static;
    use serde_json::Value;
    use tower::ServiceExt;

    lazy_static! {
        static ref INIT: Once = Once::new();
    }

    fn initialize_logging() {
        INIT.call_once(|| {
            tracing_subscriber::fmt::init();
        });
    }

    fn test_config() -> Config {
        Config {
            server_address: "0.0.0.0".into(),
            server_port: 8080,
            log_level: LogLevel::Info.into(),
            network_magic: 1,
            mode: Mode::Full,
            node_socket_path: "/run/cardano-node/node.socket".into(),
            icebreakers_config: None,
            max_pool_connections: 10,
            network: Network::Preprod,
        }
    }

    #[tokio::test]
    async fn test_root_route() {
        initialize_logging();

        let config = test_config();
        let app = match build(&config).await {
            Ok((app, _)) => app,
            Err(e) => panic!("Failed to build app: {:?}", e),
        };

        let response = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .expect("Failed to send request");

        println!("response:{:?}", response.body());

        // assert_eq!(
        //     response.status(),
        //     StatusCode::OK,
        //     "Root route should return 200"
        // );

        let body_bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("Failed to read response body");

        let json_body: Value =
            serde_json::from_slice(&body_bytes).expect("Response body is not valid JSON");

        let body_text = String::from_utf8_lossy(&body_bytes);

        println!("body_text:{:?}", body_text);

        assert_eq!(json_body["name"], "blockfrost-platform");
    }
}
