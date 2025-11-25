use crate::types::Network;
use axum::Extension;
use bf_api_provider::types::GenesisResponse;
use std::sync::Arc;

pub type GenesisExtension = Extension<Arc<Vec<(Network, GenesisResponse)>>>;

pub trait GenesisRegistry {
    /// Get a network config by its `Network` enum variant.
    fn by_network(&self, network: &Network) -> GenesisResponse;

    /// Get a network config by magic.
    fn by_magic(&self, magic: u64) -> GenesisResponse;

    /// List all known network magics.
    fn all_magics(&self) -> Vec<u64>;

    /// Map a magic number back to its `Network`.
    fn network_by_magic(&self, magic: u64) -> &Network;

    /// Insert or replace the `GenesisContent` for `network` at the front.
    fn add(&mut self, network: Network, genesis: GenesisResponse);
}

impl GenesisRegistry for Vec<(Network, GenesisResponse)> {
    fn by_network(&self, network: &Network) -> GenesisResponse {
        self.iter()
            .find(|(n, _)| n == network)
            .map(|(_, g)| g.clone())
            .expect("Missing GenesisContent for known Network")
    }

    fn by_magic(&self, magic: u64) -> GenesisResponse {
        self.iter()
            .find(|(_, g)| g.network_magic as u64 == magic)
            .map(|(_, g)| g.clone())
            .expect("Missing GenesisContent for known magic")
    }

    fn all_magics(&self) -> Vec<u64> {
        self.iter().map(|(_, g)| g.network_magic as u64).collect()
    }

    fn network_by_magic(&self, magic: u64) -> &Network {
        self.iter()
            .find(|(_, g)| g.network_magic as u64 == magic)
            .map(|(n, _)| n)
            .expect("Missing Network for known magic")
    }

    fn add(&mut self, network: Network, genesis: GenesisResponse) {
        // If the network already exists, replace its GenesisContent;
        // otherwise, insert this (network, genesis) tuple at index 0.
        if let Some((_, slot)) = self.iter_mut().find(|(n, _)| n == &network) {
            *slot = genesis;
        } else {
            self.insert(0, (network, genesis));
        }
    }
}

pub fn genesis() -> Vec<(Network, GenesisResponse)> {
    vec![
        (
            Network::Mainnet,
            GenesisResponse {
                active_slots_coefficient: 0.05,
                update_quorum: 5,
                max_lovelace_supply: "45000000000000000".to_string(),
                network_magic: 764_824_073,
                epoch_length: 432_000,
                system_start: 1_506_203_091,
                slots_per_kes_period: 129_600,
                slot_length: 1,
                max_kes_evolutions: 62,
                security_param: 2160,
            },
        ),
        (
            Network::Preprod,
            GenesisResponse {
                active_slots_coefficient: 0.05,
                update_quorum: 5,
                max_lovelace_supply: "45000000000000000".to_string(),
                network_magic: 1,
                epoch_length: 432_000,
                system_start: 1_654_041_600,
                slots_per_kes_period: 129_600,
                slot_length: 1,
                max_kes_evolutions: 62,
                security_param: 2160,
            },
        ),
        (
            Network::Preview,
            GenesisResponse {
                active_slots_coefficient: 0.05,
                update_quorum: 5,
                max_lovelace_supply: "45000000000000000".to_string(),
                network_magic: 2,
                epoch_length: 86_400,
                system_start: 1_666_692_000,
                slots_per_kes_period: 129_600,
                slot_length: 1,
                max_kes_evolutions: 62,
                security_param: 432,
            },
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case(Network::Mainnet, 764_824_073)]
    #[case(Network::Preprod, 1)]
    #[case(Network::Preview, 2)]
    fn test_by_network_returns_correct_magic(
        #[case] network: Network,
        #[case] expected_magic: i32,
    ) {
        let registry = genesis();
        let genesis = registry.by_network(&network);
        assert_eq!(genesis.network_magic, expected_magic);
    }

    #[rstest]
    #[case(764_824_073, 1_506_203_091)]
    #[case(1, 1_654_041_600)]
    #[case(2, 1_666_692_000)]
    fn test_by_magic_returns_correct_start(#[case] magic: i32, #[case] expected_start: i32) {
        let registry = genesis();
        let genesis = registry.by_magic(magic as u64);
        assert_eq!(genesis.system_start, expected_start);
    }

    #[test]
    fn test_all_magics_len_is_three() {
        let magics = genesis().all_magics();
        assert_eq!(magics.len(), 3);
    }

    #[rstest]
    #[case(764_824_073)]
    #[case(1)]
    #[case(2)]
    fn test_all_magics_contains_each(#[case] magic: u64) {
        let magics = genesis().all_magics();
        assert!(magics.contains(&magic));
    }

    #[rstest]
    #[case(764_824_073, Network::Mainnet)]
    #[case(1, Network::Preprod)]
    #[case(2, Network::Preview)]
    fn test_network_by_magic_returns_correct_network(
        #[case] magic: u64,
        #[case] expected_network: Network,
    ) {
        let registry = genesis();
        let network = registry.network_by_magic(magic);
        assert_eq!(network, &expected_network);
    }

    #[rstest]
    #[case(true)]
    #[case(false)]
    fn test_add_behavior_for_replace_and_insert(#[case] prepopulated: bool) {
        let mut registry = if prepopulated { genesis() } else { Vec::new() };

        // dummy
        let dummy = GenesisResponse {
            active_slots_coefficient: 0.1,
            update_quorum: 10,
            max_lovelace_supply: "100".to_string(),
            network_magic: 999,
            epoch_length: 1000,
            system_start: 123_456,
            slots_per_kes_period: 200,
            slot_length: 2,
            max_kes_evolutions: 3,
            security_param: 4,
        };

        if prepopulated {
            let mut existing = registry.by_network(&Network::Mainnet);
            existing.network_magic = 888;
            registry.add(Network::Mainnet, existing);
        }

        registry.add(Network::Mainnet, dummy.clone());

        assert_eq!(registry[0].0, Network::Mainnet);
        assert_eq!(registry[0].1.network_magic, 999);

        let fetched = registry.by_network(&Network::Mainnet);
        assert_eq!(fetched.network_magic, 999);

        // if we started empty it must have length 1, otherwise >=1
        if prepopulated {
            assert!(!registry.is_empty());
        } else {
            assert_eq!(registry.len(), 1);
        }
    }
}
