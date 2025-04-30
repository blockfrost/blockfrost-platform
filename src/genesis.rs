use crate::cli::Network;
use blockfrost_openapi::models::genesis_content::GenesisContent;

pub fn get_genesis_content_for(network: &Network) -> GenesisContent {
    match network {
        Network::Mainnet => GenesisContent {
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
        Network::Preview => GenesisContent {
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
        Network::Preprod => GenesisContent {
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
    }
}
