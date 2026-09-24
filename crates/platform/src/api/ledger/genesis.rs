use crate::genesis::GenesisRegistry;
use crate::{api::ApiResult, server::state::AppState};
use axum::{Json, extract::State};
use bf_api_provider::types::GenesisResponse;

pub async fn route(State(state): State<AppState>) -> ApiResult<GenesisResponse> {
    let genesis = state.config.genesis.by_network(&state.config.network);

    Ok(Json(genesis.clone()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, Mode};
    use crate::genesis::{GenesisRegistryMut, genesis};
    use crate::server::state::AppState;
    use axum::extract::State;
    use bf_common::types::{LogLevel, Network};
    use std::sync::Arc;

    fn dummy_genesis(network_magic: i32) -> GenesisResponse {
        GenesisResponse {
            active_slots_coefficient: 0.1,
            update_quorum: 7,
            max_lovelace_supply: "123456789".to_string(),
            network_magic,
            epoch_length: 100,
            system_start: 1000,
            slots_per_kes_period: 200,
            slot_length: 2,
            max_kes_evolutions: 9,
            security_param: 11,
        }
    }

    fn app_state(network: Network, registry: Vec<(Network, GenesisResponse)>) -> AppState {
        let config = Config {
            server_address: "0.0.0.0".parse().unwrap(),
            server_port: 3000,
            server_concurrency_limit: 2048,
            log_level: LogLevel::Info.into(),
            node_socket_path: "/path/to/socket".to_string(),
            mode: Mode::Compact,
            icebreakers_config: None,
            max_pool_connections: 10,
            max_response_body_bytes: bf_common::DEFAULT_MAX_BODY_BYTES,
            no_metrics: false,
            network,
            custom_genesis_config: None,
            genesis: registry,
            data_node: None,
            hydra: None,
        };

        AppState {
            config: Arc::new(config),
            data_node: None,
        }
    }

    #[tokio::test]
    async fn serves_custom_genesis_when_network_is_custom() {
        let mut registry = genesis();
        registry.add(Network::Custom, dummy_genesis(42));

        let state = app_state(Network::Custom, registry);

        let Json(response) = route(State(state)).await.expect("route should succeed");

        // The endpoint must serve the merged-in custom genesis, not a built-in.
        assert_eq!(response.network_magic, 42);
        assert_eq!(response.epoch_length, 100);
        assert_eq!(response.security_param, 11);
    }

    #[tokio::test]
    async fn serves_builtin_genesis_for_known_network() {
        let registry = genesis();
        let state = app_state(Network::Preview, registry);

        let Json(response) = route(State(state)).await.expect("route should succeed");

        assert_eq!(response.network_magic, 2);
        assert_eq!(response.epoch_length, 86_400);
    }
}
