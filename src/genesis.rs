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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::Network;

    #[test]
    fn test_by_network_returns_correct_genesis() {
        let registry = genesis();

        let mainnet = registry.by_network(&Network::Mainnet);
        assert_eq!(mainnet.network_magic, 764_824_073);

        let preprod = registry.by_network(&Network::Preprod);
        assert_eq!(preprod.network_magic, 1);

        let preview = registry.by_network(&Network::Preview);
        assert_eq!(preview.network_magic, 2);
    }

    #[test]
    fn test_by_magic_returns_correct_genesis() {
        let registry = genesis();

        let mainnet = registry.by_magic(764_824_073);
        assert_eq!(mainnet.system_start, 1_506_203_091);

        let preprod = registry.by_magic(1);
        assert_eq!(preprod.system_start, 1_654_041_600);

        let preview = registry.by_magic(2);
        assert_eq!(preview.system_start, 1_666_692_000);
    }

    #[test]
    fn test_all_magics_returns_all_magics() {
        let registry = genesis();
        let magics = registry.all_magics();

        assert_eq!(magics.len(), 3);

        assert!(magics.contains(&764_824_073));
        assert!(magics.contains(&1));
        assert!(magics.contains(&2));
    }

    #[test]
    fn test_network_by_magic_returns_correct_network() {
        let registry = genesis();

        assert_eq!(registry.network_by_magic(764_824_073), &Network::Mainnet);
        assert_eq!(registry.network_by_magic(1), &Network::Preprod);
        assert_eq!(registry.network_by_magic(2), &Network::Preview);
    }
}
