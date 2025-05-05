use crate::cli::Network;
use blockfrost_openapi::models::genesis_content::GenesisContent;

pub trait GenesisRegistry {
    fn by_network(&self, network: &Network) -> GenesisContent;
    fn by_magic(&self, magic: u64) -> GenesisContent;
    fn all_magics(&self) -> Vec<u64>;
    fn network_by_magic(&self, magic: u64) -> &Network;
}

impl GenesisRegistry for Vec<(Network, GenesisContent)> {
    fn by_network(&self, network: &Network) -> GenesisContent {
        self.iter()
            .find(|(n, _)| n == network)
            .map(|(_, g)| g.clone())
            .expect("Missing GenesisContent for known Network")
    }

    fn by_magic(&self, magic: u64) -> GenesisContent {
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
}

pub fn genesis() -> Vec<(Network, GenesisContent)> {
    vec![
        (
            Network::Mainnet,
            GenesisContent {
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
            GenesisContent {
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
            GenesisContent {
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
