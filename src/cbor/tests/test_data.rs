use pallas_primitives::{
    ExUnitPrices, ExUnits, RationalNumber,
    conway::{DRepVotingThresholds, PoolVotingThresholds},
};
use pallas_traverse::update::ConwayCostModels;
use pallas_validate::utils::{ConwayProtParams, MultiEraProtocolParameters};

pub fn get_preview_pp() -> MultiEraProtocolParameters {
    MultiEraProtocolParameters::Conway(ConwayProtParams {
        system_start: "2022-10-25T00:00:00+00:00".parse().unwrap(),
        epoch_length: 86400,
        slot_length: 1_000_000,
        minfee_a: 44,
        minfee_b: 155_381,
        max_block_body_size: 90_112,
        max_transaction_size: 16_384,
        max_block_header_size: 1_100,
        key_deposit: 2_000_000,
        pool_deposit: 500_000_000,
        desired_number_of_stake_pools: 500,
        protocol_version: (7, 0),
        min_pool_cost: 170_000_000,
        ada_per_utxo_byte: 4_310,
        cost_models_for_script_languages: ConwayCostModels {
            plutus_v1: Some(vec![
                100788, 420, 1, 1, 1000, 173, 0, 1, 1000, 59957, 4, 1, 11183, 32, 201305, 8356, 4,
                16000, 100, 16000, 100, 16000, 100, 16000, 100, 16000, 100, 16000, 100, 100, 100,
                16000, 100, 94375, 32, 132994, 32, 61462, 4, 72010, 178, 0, 1, 22151, 32, 91189,
                769, 4, 2, 85848, 228465, 122, 0, 1, 1, 1000, 42921, 4, 2, 24548, 29498, 38, 1,
                898148, 27279, 1, 51775, 558, 1, 39184, 1000, 60594, 1, 141895, 32, 83150, 32,
                15299, 32, 76049, 1, 13169, 4, 22100, 10, 28999, 74, 1, 28999, 74, 1, 43285, 552,
                1, 44749, 541, 1, 33852, 32, 68246, 32, 72362, 32, 7243, 32, 7391, 32, 11546, 32,
                85848, 228465, 122, 0, 1, 1, 90434, 519, 0, 1, 74433, 32, 85848, 228465, 122, 0, 1,
                1, 85848, 228465, 122, 0, 1, 1, 270652, 22588, 4, 1457325, 64566, 4, 20467, 1, 4,
                0, 141992, 32, 100788, 420, 1, 1, 81663, 32, 59498, 32, 20142, 32, 24588, 32,
                20744, 32, 25933, 32, 24623, 32, 53384111, 14333, 10,
            ]),
            plutus_v2: Some(vec![
                100788, 420, 1, 1, 1000, 173, 0, 1, 1000, 59957, 4, 1, 11183, 32, 201305, 8356, 4,
                16000, 100, 16000, 100, 16000, 100, 16000, 100, 16000, 100, 16000, 100, 100, 100,
                16000, 100, 94375, 32, 132994, 32, 61462, 4, 72010, 178, 0, 1, 22151, 32, 91189,
                769, 4, 2, 85848, 228465, 122, 0, 1, 1, 1000, 42921, 4, 2, 24548, 29498, 38, 1,
                898148, 27279, 1, 51775, 558, 1, 39184, 1000, 60594, 1, 141895, 32, 83150, 32,
                15299, 32, 76049, 1, 13169, 4, 22100, 10, 28999, 74, 1, 28999, 74, 1, 43285, 552,
                1, 44749, 541, 1, 33852, 32, 68246, 32, 72362, 32, 7243, 32, 7391, 32, 11546, 32,
                85848, 228465, 122, 0, 1, 1, 90434, 519, 0, 1, 74433, 32, 85848, 228465, 122, 0, 1,
                1, 85848, 228465, 122, 0, 1, 1, 955506, 213312, 0, 2, 270652, 22588, 4, 1457325,
                64566, 4, 20467, 1, 4, 0, 141992, 32, 100788, 420, 1, 1, 81663, 32, 59498, 32,
                20142, 32, 24588, 32, 20744, 32, 25933, 32, 24623, 32, 43053543, 10, 53384111,
                14333, 10, 43574283, 26308, 10,
            ]),
            plutus_v3: Some(vec![
                100788, 420, 1, 1, 1000, 173, 0, 1, 1000, 59957, 4, 1, 11183, 32, 201305, 8356, 4,
                16000, 100, 16000, 100, 16000, 100, 16000, 100, 16000, 100, 16000, 100, 100, 100,
                16000, 100, 94375, 32, 132994, 32, 61462, 4, 72010, 178, 0, 1, 22151, 32, 91189,
                769, 4, 2, 85848, 123203, 7305, -900, 1716, 549, 57, 85848, 0, 1, 1, 1000, 42921,
                4, 2, 24548, 29498, 38, 1, 898148, 27279, 1, 51775, 558, 1, 39184, 1000, 60594, 1,
                141895, 32, 83150, 32, 15299, 32, 76049, 1, 13169, 4, 22100, 10, 28999, 74, 1,
                28999, 74, 1, 43285, 552, 1, 44749, 541, 1, 33852, 32, 68246, 32, 72362, 32, 7243,
                32, 7391, 32, 11546, 32, 85848, 123203, 7305, -900, 1716, 549, 57, 85848, 0, 1,
                90434, 519, 0, 1, 74433, 32, 85848, 123203, 7305, -900, 1716, 549, 57, 85848, 0, 1,
                1, 85848, 123203, 7305, -900, 1716, 549, 57, 85848, 0, 1, 955506, 213312, 0, 2,
                270652, 22588, 4, 1457325, 64566, 4, 20467, 1, 4, 0, 141992, 32, 100788, 420, 1, 1,
                81663, 32, 59498, 32, 20142, 32, 24588, 32, 20744, 32, 25933, 32, 24623, 32,
                43053543, 10, 53384111, 14333, 10, 43574283, 26308, 10, 16000, 100, 16000, 100,
                962335, 18, 2780678, 6, 442008, 1, 52538055, 3756, 18, 267929, 18, 76433006, 8868,
                18, 52948122, 18, 1995836, 36, 3227919, 12, 901022, 1, 166917843, 4307, 36, 284546,
                36, 158221314, 26549, 36, 74698472, 36, 333849714, 1, 254006273, 72, 2174038, 72,
                2261318, 64571, 4, 207616, 8310, 4, 1293828, 28716, 63, 0, 1, 1006041, 43623, 251,
                0, 1, 100181, 726, 719, 0, 1, 100181, 726, 719, 0, 1, 100181, 726, 719, 0, 1,
                107878, 680, 0, 1, 95336, 1, 281145, 18848, 0, 1, 180194, 159, 1, 1, 158519, 8942,
                0, 1, 159378, 8813, 0, 1, 107490, 3298, 1, 106057, 655, 1, 1964219, 24520, 3,
            ]),
            unknown: Default::default(),
        },
        execution_costs: ExUnitPrices {
            mem_price: RationalNumber {
                numerator: 577,
                denominator: 10_000,
            },
            step_price: RationalNumber {
                numerator: 721,
                denominator: 10_000_000,
            },
        },
        max_tx_ex_units: ExUnits {
            mem: 14_000_000,
            steps: 10_000_000_000,
        },
        max_block_ex_units: ExUnits {
            mem: 62_000_000,
            steps: 20_000_000_000,
        },
        max_value_size: 5_000,
        collateral_percentage: 150,
        max_collateral_inputs: 3,
        expansion_rate: RationalNumber {
            numerator: 3,
            denominator: 1_000,
        },
        treasury_growth_rate: RationalNumber {
            numerator: 1,
            denominator: 5,
        },
        maximum_epoch: 18,
        pool_pledge_influence: RationalNumber {
            numerator: 3,
            denominator: 10,
        },
        pool_voting_thresholds: PoolVotingThresholds {
            motion_no_confidence: RationalNumber {
                numerator: 51,
                denominator: 100,
            },
            committee_normal: RationalNumber {
                numerator: 51,
                denominator: 100,
            },
            committee_no_confidence: RationalNumber {
                numerator: 51,
                denominator: 100,
            },
            hard_fork_initiation: RationalNumber {
                numerator: 51,
                denominator: 100,
            },
            security_voting_threshold: RationalNumber {
                numerator: 51,
                denominator: 100,
            },
        },
        drep_voting_thresholds: DRepVotingThresholds {
            motion_no_confidence: RationalNumber {
                numerator: 67,
                denominator: 100,
            },
            committee_normal: RationalNumber {
                numerator: 67,
                denominator: 100,
            },
            committee_no_confidence: RationalNumber {
                numerator: 3,
                denominator: 5,
            },
            update_constitution: RationalNumber {
                numerator: 3,
                denominator: 4,
            },
            hard_fork_initiation: RationalNumber {
                numerator: 3,
                denominator: 5,
            },
            pp_network_group: RationalNumber {
                numerator: 67,
                denominator: 100,
            },
            pp_economic_group: RationalNumber {
                numerator: 67,
                denominator: 100,
            },
            pp_technical_group: RationalNumber {
                numerator: 67,
                denominator: 100,
            },
            pp_governance_group: RationalNumber {
                numerator: 3,
                denominator: 4,
            },
            treasury_withdrawal: RationalNumber {
                numerator: 67,
                denominator: 100,
            },
        },
        min_committee_size: 0,
        committee_term_limit: 365,
        governance_action_validity_period: 30,
        governance_action_deposit: 100_000_000_000,
        drep_deposit: 500_000_000,
        drep_inactivity_period: 31,
        minfee_refscript_cost_per_byte: RationalNumber {
            numerator: 15,
            denominator: 1,
        },
    })
}
