use std::collections::BTreeMap;
use std::sync::LazyLock;

use chrono::DateTime;
use chrono::NaiveDate;
use chrono::NaiveDateTime;
use chrono::NaiveTime;
use pallas_network::miniprotocols::localstate::queries_v16::GenesisConfig;
use pallas_network::miniprotocols::localstate::queries_v16::ProtocolParam;
use pallas_network::miniprotocols::localstate::queries_v16::SystemStart;
use pallas_primitives::ExUnitPrices;
use pallas_primitives::ExUnits;
use pallas_primitives::RationalNumber;
use pallas_primitives::conway;
use pallas_traverse::MultiEraTx;
use pallas_validate::{
    phase2::{EvalReport, script_context::SlotConfig},
    utils::{ConwayProtParams, MultiEraProtocolParameters, UtxoMap},
};

use crate::BlockfrostError;
use crate::NodePool;

static EMPY_UTXOS: LazyLock<UtxoMap> = LazyLock::new(UtxoMap::new);

pub async fn evaluate_binary_tx(
    node_pool: NodePool,
    tx_cbor_binary: &[u8],
    utxos: Option<&UtxoMap>,
) -> Result<EvalReport, BlockfrostError> {
    let minted_tx: conway::Tx =
        pallas_codec::minicbor::decode::<conway::Tx>(tx_cbor_binary).unwrap();
    let multi_era_tx: MultiEraTx = MultiEraTx::from_conway(&minted_tx);
    let slot_config = pallas_validate::phase2::script_context::SlotConfig::default();

    evaluate_tx(
        node_pool,
        &multi_era_tx,
        &slot_config,
        utxos.unwrap_or(&EMPY_UTXOS),
    )
    .await
}

pub async fn evaluate_encoded_tx(
    node_pool: NodePool,
    tx_cbor: &String,
    utxos: Option<&UtxoMap>,
) -> Result<EvalReport, BlockfrostError> {
    let tx_cbor_binary = hex::decode(tx_cbor).unwrap();

    evaluate_binary_tx(node_pool, &tx_cbor_binary, utxos).await
}

pub async fn evaluate_tx(
    node_pool: NodePool,
    tx: &MultiEraTx<'_>,
    slot_config: &SlotConfig,
    utxos: &UtxoMap,
) -> Result<EvalReport, BlockfrostError> {
    let mut node: deadpool::managed::Object<crate::node::pool_manager::NodePoolManager> =
        node_pool.get().await?;

    let genesis_config: GenesisConfig = node.genesis_config().await?;
    let protocol_params: MultiEraProtocolParameters =
        convert_protocol_param(node.protocol_params().await?, genesis_config);

    match evaluate_tx_with(tx, &protocol_params, utxos, slot_config) {
        Ok(report) => Ok(report),
        Err(e) => Err(BlockfrostError::custom_400(
            "Error evaluating transaction: ".to_string() + &e.to_string(),
        )),
    }
}

// Implemented in this way for easy unit testing
fn evaluate_tx_with(
    tx: &MultiEraTx<'_>,
    protocol_params: &MultiEraProtocolParameters,
    utxos: &UtxoMap,
    slot_config: &SlotConfig,
) -> Result<EvalReport, BlockfrostError> {
    match pallas_validate::phase2::evaluate_tx(tx, protocol_params, utxos, slot_config) {
        Ok(report) => Ok(report),
        Err(e) => Err(BlockfrostError::custom_400(
            "Error evaluating transaction: ".to_string() + &e.to_string(),
        )),
    }
}

fn convert_system_start(sys_start: SystemStart) -> chrono::DateTime<chrono::FixedOffset> {
    let naive_date = NaiveDate::from_yo_opt(sys_start.year as i32, sys_start.day_of_year)
        .expect("Invalid system start date");

    let secs = (sys_start.picoseconds_of_day / 1_000_000_000_000) as u32;
    let nano = ((sys_start.picoseconds_of_day % 1_000_000_000_000) / 1000) as u32;

    let naive_time =
        NaiveTime::from_num_seconds_from_midnight_opt(secs, nano).expect("Invalid time");

    let naive_date_time = NaiveDateTime::new(naive_date, naive_time);

    // Convert to DateTime with UTC timezone (zero offset)
    let utc_offset = chrono::FixedOffset::east_opt(0).expect("Invalid offset");

    DateTime::from_naive_utc_and_offset(naive_date_time, utc_offset)
}

fn convert_protocol_param(pp: ProtocolParam, genesis: GenesisConfig) -> MultiEraProtocolParameters {
    // Create a Conway protocol parameters struct from the ProtocolParam and GenesisConfig
    let conway_pp: ConwayProtParams = ConwayProtParams {
        minfee_a: pp.minfee_a.unwrap() as u32,
        minfee_b: pp.minfee_b.unwrap() as u32,
        max_block_body_size: pp.max_block_body_size.unwrap() as u32,
        max_transaction_size: pp.max_transaction_size.unwrap() as u32,
        max_block_header_size: pp.max_block_header_size.unwrap() as u32,
        system_start: convert_system_start(genesis.system_start),
        epoch_length: genesis.epoch_length as u64,
        slot_length: genesis.slot_length as u64,
        desired_number_of_stake_pools: pp.desired_number_of_stake_pools.unwrap() as u32,
        protocol_version: (7, 0), // hardcoded,
        ada_per_utxo_byte: pp.ada_per_utxo_byte.unwrap().into(),
        cost_models_for_script_languages: {
            let models = pp.cost_models_for_script_languages.unwrap();
            conway::CostModels {
                plutus_v1: models.plutus_v1,
                plutus_v2: models.plutus_v2,
                plutus_v3: models.plutus_v3,
                unknown: BTreeMap::new(), // unknowns doesn't matter
            }
        },
        execution_costs: {
            let costs = pp.execution_costs.unwrap();
            ExUnitPrices {
                mem_price: RationalNumber {
                    numerator: costs.mem_price.numerator,
                    denominator: costs.mem_price.denominator,
                },
                step_price: RationalNumber {
                    numerator: costs.step_price.numerator,
                    denominator: costs.step_price.denominator,
                },
            }
        },
        max_tx_ex_units: {
            let ex_units = pp.max_tx_ex_units.unwrap();
            ExUnits {
                mem: ex_units.mem,
                steps: ex_units.steps,
            }
        },
        max_block_ex_units: {
            let ex_units = pp.max_block_ex_units.unwrap();
            ExUnits {
                mem: ex_units.mem,
                steps: ex_units.steps,
            }
        },
        max_value_size: pp.max_value_size.unwrap() as u32,
        expansion_rate: {
            let expansion_rate = pp.expansion_rate.unwrap();
            RationalNumber {
                numerator: expansion_rate.numerator,
                denominator: expansion_rate.denominator,
            }
        },
        treasury_growth_rate: {
            let growth_rate = pp.treasury_growth_rate.unwrap();
            RationalNumber {
                numerator: growth_rate.numerator,
                denominator: growth_rate.denominator,
            }
        },
        maximum_epoch: pp.maximum_epoch.unwrap(),
        pool_pledge_influence: {
            let influence = pp.pool_pledge_influence.unwrap();
            RationalNumber {
                numerator: influence.numerator,
                denominator: influence.denominator,
            }
        },
        pool_voting_thresholds: {
            let thresholds = pp.pool_voting_thresholds.unwrap();
            conway::PoolVotingThresholds {
                motion_no_confidence: RationalNumber {
                    numerator: thresholds.motion_no_confidence.numerator,
                    denominator: thresholds.motion_no_confidence.denominator,
                },
                committee_normal: RationalNumber {
                    numerator: thresholds.committee_normal.numerator,
                    denominator: thresholds.committee_normal.denominator,
                },
                committee_no_confidence: RationalNumber {
                    numerator: thresholds.committee_no_confidence.numerator,
                    denominator: thresholds.committee_no_confidence.denominator,
                },
                hard_fork_initiation: RationalNumber {
                    numerator: thresholds.hard_fork_initiation.numerator,
                    denominator: thresholds.hard_fork_initiation.denominator,
                },
                security_voting_threshold: RationalNumber {
                    numerator: thresholds.pp_security_group.numerator,
                    denominator: thresholds.pp_security_group.denominator,
                },
            }
        },
        drep_voting_thresholds: {
            let thresholds = pp.drep_voting_thresholds.unwrap();
            conway::DRepVotingThresholds {
                motion_no_confidence: RationalNumber {
                    numerator: thresholds.motion_no_confidence.numerator,
                    denominator: thresholds.motion_no_confidence.denominator,
                },
                committee_normal: RationalNumber {
                    numerator: thresholds.committee_normal.numerator,
                    denominator: thresholds.committee_normal.denominator,
                },
                committee_no_confidence: RationalNumber {
                    numerator: thresholds.committee_no_confidence.numerator,
                    denominator: thresholds.committee_no_confidence.denominator,
                },
                update_constitution: RationalNumber {
                    numerator: thresholds.update_to_constitution.numerator,
                    denominator: thresholds.update_to_constitution.denominator,
                },
                hard_fork_initiation: RationalNumber {
                    numerator: thresholds.hard_fork_initiation.numerator,
                    denominator: thresholds.hard_fork_initiation.denominator,
                },
                pp_network_group: RationalNumber {
                    numerator: thresholds.pp_network_group.numerator,
                    denominator: thresholds.pp_network_group.denominator,
                },
                pp_economic_group: RationalNumber {
                    numerator: thresholds.pp_economic_group.numerator,
                    denominator: thresholds.pp_economic_group.denominator,
                },
                pp_technical_group: RationalNumber {
                    numerator: thresholds.pp_technical_group.numerator,
                    denominator: thresholds.pp_technical_group.denominator,
                },
                pp_governance_group: RationalNumber {
                    numerator: thresholds.pp_gov_group.numerator,
                    denominator: thresholds.pp_gov_group.denominator,
                },
                treasury_withdrawal: RationalNumber {
                    numerator: thresholds.treasury_withdrawal.numerator,
                    denominator: thresholds.treasury_withdrawal.denominator,
                },
            }
        },
        minfee_refscript_cost_per_byte: {
            let minfee_refscript_cost_per_byte = pp.minfee_refscript_cost_per_byte.unwrap();
            RationalNumber {
                numerator: minfee_refscript_cost_per_byte.numerator,
                denominator: minfee_refscript_cost_per_byte.denominator,
            }
        },
        key_deposit: pp.key_deposit.unwrap().into(),
        pool_deposit: pp.pool_deposit.unwrap().into(),
        min_pool_cost: pp.min_pool_cost.unwrap().into(),
        collateral_percentage: pp.collateral_percentage.unwrap() as u32,
        max_collateral_inputs: pp.max_collateral_inputs.unwrap() as u32,
        min_committee_size: pp.min_committee_size.unwrap(),
        committee_term_limit: pp.committee_term_limit.unwrap(),
        governance_action_validity_period: pp.governance_action_validity_period.unwrap(),
        governance_action_deposit: pp.governance_action_deposit.unwrap().into(),
        drep_deposit: pp.drep_deposit.unwrap().into(),
        drep_inactivity_period: pp.drep_inactivity_period.unwrap(),
    };

    MultiEraProtocolParameters::Conway(conway_pp)
}

#[cfg(test)]
mod tests {
    use std::{borrow::Cow, iter::zip};

    use chrono::{Datelike, Timelike};
    use pallas_codec::utils::Bytes;
    use pallas_codec::utils::{AnyUInt, CborWrap};
    use pallas_network::miniprotocols::localstate::queries_v16::{
        self, Fraction,
    };
    use pallas_primitives::conway::{DRepVotingThresholds, PoolVotingThresholds};
    use pallas_primitives::{
        KeepRaw, PlutusScript,
        conway::{DatumOption, ScriptRef, Value},
    };
    use pallas_traverse::{MultiEraInput, MultiEraOutput};
    use pallas_validate::utils::{EraCbor, TxoRef, UTxOs};

    use super::*;

    #[test]
    fn test_convert_system_start() {
        // Test case 1: Standard date
        let sys_start = SystemStart {
            year: 2023,
            day_of_year: 100,
            picoseconds_of_day: 43_200_000_000_000_000, // 12 hours
        };
        let result = convert_system_start(sys_start);
        assert_eq!(result.year(), 2023);
        assert_eq!(result.ordinal(), 100);
        assert_eq!(result.hour(), 12);
        assert_eq!(result.minute(), 0);
        assert_eq!(result.second(), 0);

        // Test case 2: Year boundary
        let sys_start = SystemStart {
            year: 2024,
            day_of_year: 1,
            picoseconds_of_day: 3_600_000_000_000_000, // 1 hour
        };
        let result = convert_system_start(sys_start);
        assert_eq!(result.year(), 2024);
        assert_eq!(result.ordinal(), 1);
        assert_eq!(result.hour(), 1);
        assert_eq!(result.minute(), 0);
        assert_eq!(result.second(), 0);

        // Test case 3: Leap year
        let sys_start = SystemStart {
            year: 2024,
            day_of_year: 366,
            picoseconds_of_day: 0, // midnight
        };
        let result = convert_system_start(sys_start);
        assert_eq!(result.year(), 2024);
        assert_eq!(result.ordinal(), 366);
        assert_eq!(result.hour(), 0);
        assert_eq!(result.minute(), 0);
        assert_eq!(result.second(), 0);

        // Test case 4: Partial seconds
        let sys_start = SystemStart {
            year: 2023,
            day_of_year: 200,
            picoseconds_of_day: 63_123_000_000_000_000, // 17:32:03
        };
        let result = convert_system_start(sys_start);
        assert_eq!(result.year(), 2023);
        assert_eq!(result.ordinal(), 200);
        assert_eq!(result.hour(), 17);
        assert_eq!(result.minute(), 32);
        assert_eq!(result.second(), 3);
    }

    #[test]
    fn test_convert_protocol_param() {
        // Create minimal test data with required fields
        let genesis = GenesisConfig {
            system_start: SystemStart {
                year: 2023,
                day_of_year: 100,
                picoseconds_of_day: 43_200_000_000_000_000, // 12 hours
            },
            epoch_length: 432000,
            slot_length: 1,
            max_lovelace_supply: AnyUInt::U64(45_000_000_000_000_000),
            security_param: 2160,
            active_slots_coefficient: Fraction {
                num: 618,
                den: 1000,
            },
            network_id: 1,
            network_magic: 13,
            slots_per_kes_period: 2160,
            max_kes_evolutions: 62,
            update_quorum: 5,
        };
        let pp = ProtocolParam {
            minfee_a: Some(44),
            minfee_b: Some(155381),
            max_block_body_size: Some(90112),
            max_transaction_size: Some(16384),
            max_block_header_size: Some(1100),
            key_deposit: Some(AnyUInt::U64(2_000_000)),
            pool_deposit: Some(AnyUInt::U64(500_000_000)),
            min_pool_cost: Some(AnyUInt::U64(340_000_000)),
            desired_number_of_stake_pools: Some(500),
            ada_per_utxo_byte: Some(AnyUInt::U64(4310)),
            collateral_percentage: Some(150),
            max_collateral_inputs: Some(3),
            cost_models_for_script_languages: Some(queries_v16::CostModels {
                plutus_v1: Some(vec![1, 2, 3]),
                plutus_v2: Some(vec![4, 5, 6]),
                plutus_v3: Some(vec![7, 8, 9]),
                unknown: [].to_vec().into(),
            }),
            execution_costs: Some(queries_v16::ExUnitPrices {
                mem_price: queries_v16::PositiveInterval {
                    numerator: 577,
                    denominator: 10_000,
                },
                step_price: queries_v16::PositiveInterval {
                    numerator: 721,
                    denominator: 10_000_000,
                },
            }),
            max_tx_ex_units: Some(queries_v16::ExUnits {
                mem: 14_000_000,
                steps: 10_000_000_000,
            }),
            max_block_ex_units: Some(queries_v16::ExUnits {
                mem: 62_000_000,
                steps: 40_000_000_000,
            }),
            max_value_size: Some(5000),
            expansion_rate: Some(queries_v16::RationalNumber {
                numerator: 3,
                denominator: 1000,
            }),
            treasury_growth_rate: Some(queries_v16::RationalNumber {
                numerator: 20,
                denominator: 100,
            }),
            maximum_epoch: Some(18),
            pool_pledge_influence: Some(queries_v16::RationalNumber {
                numerator: 3,
                denominator: 10,
            }),
            pool_voting_thresholds: Some(queries_v16::PoolVotingThresholds {
                motion_no_confidence: queries_v16::RationalNumber {
                    numerator: 51,
                    denominator: 100,
                },
                committee_normal: queries_v16::RationalNumber {
                    numerator: 51,
                    denominator: 100,
                },
                committee_no_confidence: queries_v16::RationalNumber {
                    numerator: 51,
                    denominator: 100,
                },
                hard_fork_initiation: queries_v16::RationalNumber {
                    numerator: 51,
                    denominator: 100,
                },
                pp_security_group: queries_v16::RationalNumber {
                    numerator: 51,
                    denominator: 100,
                },
            }),
            drep_voting_thresholds: Some(queries_v16::DRepVotingThresholds {
                motion_no_confidence: queries_v16::RationalNumber {
                    numerator: 51,
                    denominator: 100,
                },
                committee_normal: queries_v16::RationalNumber {
                    numerator: 51,
                    denominator: 100,
                },
                committee_no_confidence: queries_v16::RationalNumber {
                    numerator: 51,
                    denominator: 100,
                },
                update_to_constitution: queries_v16::RationalNumber {
                    numerator: 51,
                    denominator: 100,
                },
                hard_fork_initiation: queries_v16::RationalNumber {
                    numerator: 51,
                    denominator: 100,
                },
                pp_network_group: queries_v16::RationalNumber {
                    numerator: 51,
                    denominator: 100,
                },
                pp_economic_group: queries_v16::RationalNumber {
                    numerator: 51,
                    denominator: 100,
                },
                pp_technical_group: queries_v16::RationalNumber {
                    numerator: 51,
                    denominator: 100,
                },
                pp_gov_group: queries_v16::RationalNumber {
                    numerator: 51,
                    denominator: 100,
                },
                treasury_withdrawal: queries_v16::RationalNumber {
                    numerator: 51,
                    denominator: 100,
                },
            }),
            minfee_refscript_cost_per_byte: Some(queries_v16::RationalNumber {
                numerator: 156,
                denominator: 1,
            }),
            min_committee_size: Some(3),
            committee_term_limit: Some(4),
            governance_action_deposit: Some(AnyUInt::U64(1_000_000)),
            drep_deposit: Some(AnyUInt::U64(2_000_000)),
            drep_inactivity_period: Some(3),
            governance_action_validity_period: Some(10),
        };

        // Convert protocol parameters
        let result = convert_protocol_param(pp, genesis);

        // Verify the result is a Conway protocol parameters
        if let MultiEraProtocolParameters::Conway(conway_pp) = result {
            // Verify a few fields to ensure conversion is correct
            assert_eq!(conway_pp.minfee_a, 44);
            assert_eq!(conway_pp.minfee_b, 155381);
            assert_eq!(conway_pp.max_block_body_size, 90112);
            assert_eq!(conway_pp.max_transaction_size, 16384);
            assert_eq!(conway_pp.epoch_length, 432000);
            assert_eq!(conway_pp.slot_length, 1);
            assert_eq!(conway_pp.key_deposit, 2_000_000);
            assert_eq!(conway_pp.pool_deposit, 500_000_000);
            assert_eq!(conway_pp.min_pool_cost, 340_000_000);
            assert_eq!(conway_pp.protocol_version, (7, 0));

            // Check cost models
            assert_eq!(
                conway_pp.cost_models_for_script_languages.plutus_v1,
                Some(vec![1, 2, 3])
            );
            assert_eq!(
                conway_pp.cost_models_for_script_languages.plutus_v2,
                Some(vec![4, 5, 6])
            );
            assert_eq!(
                conway_pp.cost_models_for_script_languages.plutus_v3,
                Some(vec![7, 8, 9])
            );

            // Check execution costs
            assert_eq!(conway_pp.execution_costs.mem_price.numerator, 577);
            assert_eq!(conway_pp.execution_costs.mem_price.denominator, 10_000);
            assert_eq!(conway_pp.execution_costs.step_price.numerator, 721);
            assert_eq!(conway_pp.execution_costs.step_price.denominator, 10_000_000);

            // Check governance parameters
            assert_eq!(conway_pp.governance_action_deposit, 1_000_000);
            assert_eq!(conway_pp.drep_deposit, 2_000_000);
            assert_eq!(conway_pp.min_committee_size, 3);
            assert_eq!(conway_pp.committee_term_limit, 4);
        } else {
            panic!("Expected Conway protocol parameters");
        }
    }
    #[test]
    fn test_evaluate_tx_with() {
        let tx_cbor = "84A300D90102818258203F62DBE3279603D26F4E54728E6F10CDC479974F1F6D94C32FE39A0689EFA981000182825839015C5C318D01F729E205C95EB1B02D623DD10E78EA58F72D0C13F892B2E8904EDC699E2F0CE7B72BE7CEC991DF651A222E2AE9244EB5975CBA1A00989680825839015C5C318D01F729E205C95EB1B02D623DD10E78EA58F72D0C13F892B2E8904EDC699E2F0CE7B72BE7CEC991DF651A222E2AE9244EB5975CBA1A004C4B40021A004C4B40A100D90102818258202A60DCFFE8BA15307556DBF8D7DF142CB9EB15D601251D400D523689D575B83858407E960DAAED14C00888F032D6F08F1EE5DA643330BCDFAC01406662442A8530D83B383BAF2AECD6F715028BBA5A92B5A394C0AD3FB95A43CA7A8D4705C8152204F5F6";
        let tx_cbor = hex::decode(tx_cbor).unwrap();
        let mtx: conway::Tx = pallas_codec::minicbor::decode::<conway::Tx>(&tx_cbor).unwrap();
        let metx: MultiEraTx = MultiEraTx::from_conway(&mtx);
        let datum_bytes = hex::decode("d8799f4568656c6c6fff").unwrap();
        let datum_option = DatumOption::Data(CborWrap(
            pallas_codec::minicbor::decode(&datum_bytes).unwrap(),
        ));
        let datum_option = pallas_codec::minicbor::to_vec(datum_option).unwrap();
        let datum_option: KeepRaw<'_, DatumOption> =
            pallas_codec::minicbor::decode(&datum_option).unwrap();

        let mut tx_outs_info: Vec<(
            String,
            Value,
            Option<KeepRaw<'_, DatumOption>>,
            Option<CborWrap<ScriptRef>>,
            Vec<u8>,
        )> = vec![(
            String::from("71faae60072c45d121b6e58ae35c624693ee3dad9ea8ed765eb6f76f9f"),
            Value::Coin(2000000),
            Some(datum_option),
            None,
            Vec::new(),
        )];

        let mut utxos: UTxOs =
            mk_codec_safe_utxo_for_conway_tx(&mtx.transaction_body, &mut tx_outs_info);

        let mut ref_info: Vec<(
                String,
                Value,
                Option<KeepRaw<'_, DatumOption>>,
                Option<CborWrap<ScriptRef>>,
                Vec<u8>,
            )> = vec![
                (
                    String::from("71faae60072c45d121b6e58ae35c624693ee3dad9ea8ed765eb6f76f9f"),
                    Value::Coin(1624870),
                    None,
                    Some(CborWrap(ScriptRef::PlutusV3Script(PlutusScript::<3>(Bytes::from(hex::decode("58a701010032323232323225333002323232323253330073370e900118041baa0011323322533300a3370e900018059baa00513232533300f30110021533300c3370e900018069baa00313371e6eb8c040c038dd50039bae3010300e37546020601c6ea800c5858dd7180780098061baa00516300c001300c300d001300937540022c6014601600660120046010004601000260086ea8004526136565734aae7555cf2ab9f5742ae89").unwrap()))))),
                    Vec::new(),
                ),
            ];

        add_codec_safe_ref_input_conway(&mtx.transaction_body, &mut utxos, &mut ref_info);

        let mut collateral_info: Vec<(
            String,
            Value,
            Option<KeepRaw<'_, DatumOption>>,
            Option<CborWrap<ScriptRef>>,
            Vec<u8>,
        )> = vec![(
            String::from(
                "015c5c318d01f729e205c95eb1b02d623dd10e78ea58f72d0c13f892b2e8904edc699e2f0ce7b72be7cec991df651a222e2ae9244eb5975cba",
            ),
            Value::Coin(49731771),
            None,
            None,
            Vec::new(),
        )];

        add_codec_safe_collateral_conway(&mtx.transaction_body, &mut utxos, &mut collateral_info);

        let protocol_params = MultiEraProtocolParameters::Conway(get_mainnet_params_epoch_380());

        let result = evaluate_tx_with(
            &metx,
            &protocol_params,
            &mk_utxo_for_eval(utxos.clone()),
            &pallas_validate::phase2::script_context::SlotConfig::default(),
        );

        assert!(result.is_ok(), "Tx evaluation failed");
    }

    fn mk_utxo_for_eval<'a>(utxos: UTxOs) -> UtxoMap {
        let mut eval_utxos: UtxoMap = UtxoMap::new();

        for (tx_in, tx_out) in utxos {
            eval_utxos.insert(TxoRef::from(&tx_in), EraCbor::from(tx_out));
        }
        eval_utxos
    }
    fn mk_codec_safe_utxo_for_conway_tx<'a>(
        tx_body: &pallas_primitives::conway::TransactionBody,
        tx_outs_info: &'a mut Vec<(
            String, // address in string format
            pallas_primitives::conway::Value,
            Option<pallas_codec::utils::KeepRaw<'a, pallas_primitives::conway::DatumOption>>,
            Option<CborWrap<pallas_primitives::conway::ScriptRef>>,
            Vec<u8>, // Placeholder for CBOR data.
        )>,
    ) -> UTxOs<'a> {
        let mut utxos: UTxOs = UTxOs::new();

        for (tx_in, (addr, val, datum_opt, script_ref, cbor)) in
            zip(tx_body.inputs.clone().to_vec(), tx_outs_info)
        {
            let multi_era_in: MultiEraInput =
                MultiEraInput::AlonzoCompatible(Box::new(Cow::Owned(tx_in)));
            let address_bytes: pallas_codec::utils::Bytes = match hex::decode(addr) {
                Ok(bytes_vec) => pallas_codec::utils::Bytes::from(bytes_vec),
                _ => panic!("Unable to decode input address"),
            };
            let post_alonzo = pallas_primitives::conway::PostAlonzoTransactionOutput {
                address: address_bytes,
                value: val.clone(),
                datum_option: datum_opt.clone(),
                script_ref: script_ref.clone(),
            };
            *cbor = pallas_codec::minicbor::to_vec(post_alonzo).unwrap();
            let post_alonzo = pallas_codec::minicbor::decode::<
                pallas_codec::utils::KeepRaw<
                    'a,
                    pallas_primitives::conway::PostAlonzoTransactionOutput,
                >,
            >(cbor)
            .unwrap();
            let tx_out = pallas_primitives::conway::TransactionOutput::PostAlonzo(post_alonzo);
            let multi_era_out: MultiEraOutput =
                MultiEraOutput::Conway(Box::new(Cow::Owned(tx_out)));
            utxos.insert(multi_era_in, multi_era_out);
        }
        utxos
    }

    fn add_codec_safe_ref_input_conway<'a>(
        tx_body: &pallas_primitives::conway::TransactionBody,
        utxos: &mut UTxOs<'a>,
        ref_input_info: &'a mut Vec<(
            String, // address in string format
            pallas_primitives::conway::Value,
            Option<pallas_codec::utils::KeepRaw<'a, pallas_primitives::conway::DatumOption>>,
            Option<CborWrap<pallas_primitives::conway::ScriptRef>>,
            Vec<u8>, // Placeholder for CBOR data.
        )>,
    ) {
        match &tx_body.reference_inputs {
            Some(ref_inputs) => {
                if ref_inputs.is_empty() {
                    panic!("UTxO addition error - reference input missing")
                } else {
                    for (tx_in, (addr, val, datum_opt, script_ref, cbor)) in
                        zip(ref_inputs.clone().to_vec(), ref_input_info)
                    {
                        let multi_era_in: MultiEraInput =
                            MultiEraInput::AlonzoCompatible(Box::new(Cow::Owned(tx_in)));
                        let address_bytes: Bytes = match hex::decode(addr) {
                            Ok(bytes_vec) => Bytes::from(bytes_vec),
                            _ => panic!("Unable to decode input address"),
                        };
                        let post_alonzo = pallas_primitives::conway::PostAlonzoTransactionOutput {
                            address: address_bytes,
                            value: val.clone(),
                            datum_option: datum_opt.clone(),
                            script_ref: script_ref.clone(),
                        };
                        *cbor = pallas_codec::minicbor::to_vec(post_alonzo).unwrap();
                        let post_alonzo = pallas_codec::minicbor::decode::<
                            pallas_codec::utils::KeepRaw<
                                'a,
                                pallas_primitives::conway::PostAlonzoTransactionOutput,
                            >,
                        >(cbor)
                        .unwrap();
                        let tx_out =
                            pallas_primitives::conway::TransactionOutput::PostAlonzo(post_alonzo);
                        let multi_era_out: MultiEraOutput =
                            MultiEraOutput::Conway(Box::new(Cow::Owned(tx_out)));
                        utxos.insert(multi_era_in, multi_era_out);
                    }
                }
            },
            None => panic!("UTxO addition error - reference input missing"),
        }
    }

    fn add_codec_safe_collateral_conway<'a>(
        tx_body: &pallas_primitives::conway::TransactionBody,
        utxos: &mut UTxOs<'a>,
        collateral_info: &'a mut Vec<(
            String, // address in string format
            pallas_primitives::conway::Value,
            Option<pallas_codec::utils::KeepRaw<'a, pallas_primitives::conway::DatumOption>>,
            Option<CborWrap<pallas_primitives::conway::ScriptRef>>,
            Vec<u8>, // Placeholder for CBOR data.
        )>,
    ) {
        match &tx_body.collateral {
            Some(collaterals) => {
                if collaterals.is_empty() {
                    panic!("UTxO addition error - collateral input missing")
                } else {
                    for (tx_in, (addr, val, datum_opt, script_ref, cbor)) in
                        zip(collaterals.clone().to_vec(), collateral_info)
                    {
                        let multi_era_in: MultiEraInput =
                            MultiEraInput::AlonzoCompatible(Box::new(Cow::Owned(tx_in)));
                        let address_bytes: Bytes = match hex::decode(addr) {
                            Ok(bytes_vec) => Bytes::from(bytes_vec),
                            _ => panic!("Unable to decode input address"),
                        };
                        let post_alonzo = pallas_primitives::conway::PostAlonzoTransactionOutput {
                            address: address_bytes,
                            value: val.clone(),
                            datum_option: datum_opt.clone(),
                            script_ref: script_ref.clone(),
                        };
                        *cbor = pallas_codec::minicbor::to_vec(post_alonzo).unwrap();
                        let post_alonzo = pallas_codec::minicbor::decode::<
                            pallas_codec::utils::KeepRaw<
                                'a,
                                pallas_primitives::conway::PostAlonzoTransactionOutput,
                            >,
                        >(cbor)
                        .unwrap();
                        let tx_out =
                            pallas_primitives::conway::TransactionOutput::PostAlonzo(post_alonzo);
                        let multi_era_out: MultiEraOutput =
                            MultiEraOutput::Conway(Box::new(Cow::Owned(tx_out)));
                        utxos.insert(multi_era_in, multi_era_out);
                    }
                }
            },
            None => panic!("UTxO addition error - collateral input missing"),
        }
    }

    fn get_mainnet_params_epoch_380() -> ConwayProtParams {
        ConwayProtParams {
            system_start: chrono::DateTime::parse_from_rfc3339("2022-10-25T00:00:00Z").unwrap(),
            epoch_length: 432000,
            slot_length: 1,
            minfee_a: 44,
            minfee_b: 155381,
            max_block_body_size: 90112,
            max_transaction_size: 16384,
            max_block_header_size: 1100,
            key_deposit: 2000000,
            pool_deposit: 500000000,
            maximum_epoch: 18,
            desired_number_of_stake_pools: 500,
            pool_pledge_influence: RationalNumber {
                numerator: 3,
                denominator: 10,
            },
            expansion_rate: RationalNumber {
                numerator: 3,
                denominator: 1000,
            },
            treasury_growth_rate: RationalNumber {
                numerator: 2,
                denominator: 10,
            },
            protocol_version: (7, 0),
            min_pool_cost: 340000000,
            ada_per_utxo_byte: 4310,
            cost_models_for_script_languages: pallas_primitives::conway::CostModels {
                plutus_v1: Some(vec![
                    205665, 812, 1, 1, 1000, 571, 0, 1, 1000, 24177, 4, 1, 1000, 32, 117366, 10475,
                    4, 23000, 100, 23000, 100, 23000, 100, 23000, 100, 23000, 100, 23000, 100, 100,
                    100, 23000, 100, 19537, 32, 175354, 32, 46417, 4, 221973, 511, 0, 1, 89141, 32,
                    497525, 14068, 4, 2, 196500, 453240, 220, 0, 1, 1, 1000, 28662, 4, 2, 245000,
                    216773, 62, 1, 1060367, 12586, 1, 208512, 421, 1, 187000, 1000, 52998, 1,
                    80436, 32, 43249, 32, 1000, 32, 80556, 1, 57667, 4, 1000, 10, 197145, 156, 1,
                    197145, 156, 1, 204924, 473, 1, 208896, 511, 1, 52467, 32, 64832, 32, 65493,
                    32, 22558, 32, 16563, 32, 76511, 32, 196500, 453240, 220, 0, 1, 1, 69522,
                    11687, 0, 1, 60091, 32, 196500, 453240, 220, 0, 1, 1, 196500, 453240, 220, 0,
                    1, 1, 806990, 30482, 4, 1927926, 82523, 4, 265318, 0, 4, 0, 85931, 32, 205665,
                    812, 1, 1, 41182, 32, 212342, 32, 31220, 32, 32696, 32, 43357, 32, 32247, 32,
                    38314, 32, 9462713, 1021, 10,
                ]),

                plutus_v2: Some(vec![
                    205665,
                    812,
                    1,
                    1,
                    1000,
                    571,
                    0,
                    1,
                    1000,
                    24177,
                    4,
                    1,
                    1000,
                    32,
                    117366,
                    10475,
                    4,
                    23000,
                    100,
                    23000,
                    100,
                    23000,
                    100,
                    23000,
                    100,
                    23000,
                    100,
                    23000,
                    100,
                    100,
                    100,
                    23000,
                    100,
                    19537,
                    32,
                    175354,
                    32,
                    46417,
                    4,
                    221973,
                    511,
                    0,
                    1,
                    89141,
                    32,
                    497525,
                    14068,
                    4,
                    2,
                    196500,
                    453240,
                    220,
                    0,
                    1,
                    1,
                    1000,
                    28662,
                    4,
                    2,
                    245000,
                    216773,
                    62,
                    1,
                    1060367,
                    12586,
                    1,
                    208512,
                    421,
                    1,
                    187000,
                    1000,
                    52998,
                    1,
                    80436,
                    32,
                    43249,
                    32,
                    1000,
                    32,
                    80556,
                    1,
                    57667,
                    4,
                    1000,
                    10,
                    197145,
                    156,
                    1,
                    197145,
                    156,
                    1,
                    204924,
                    473,
                    1,
                    208896,
                    511,
                    1,
                    52467,
                    32,
                    64832,
                    32,
                    65493,
                    32,
                    22558,
                    32,
                    16563,
                    32,
                    76511,
                    32,
                    196500,
                    453240,
                    220,
                    0,
                    1,
                    1,
                    69522,
                    11687,
                    0,
                    1,
                    60091,
                    32,
                    196500,
                    453240,
                    220,
                    0,
                    1,
                    1,
                    196500,
                    453240,
                    220,
                    0,
                    1,
                    1,
                    1159724,
                    392670,
                    0,
                    2,
                    806990,
                    30482,
                    4,
                    1927926,
                    82523,
                    4,
                    265318,
                    0,
                    4,
                    0,
                    85931,
                    32,
                    205665,
                    812,
                    1,
                    1,
                    41182,
                    32,
                    212342,
                    32,
                    31220,
                    32,
                    32696,
                    32,
                    43357,
                    32,
                    32247,
                    32,
                    38314,
                    32,
                    20000000000,
                    20000000000,
                    9462713,
                    1021,
                    10,
                    20000000000,
                    0,
                    20000000000,
                ]),
                plutus_v3: Some(vec![
                    100788, 420, 1, 1, 1000, 173, 0, 1, 1000, 59957, 4, 1, 11183, 32, 201305, 8356,
                    4, 16000, 100, 16000, 100, 16000, 100, 16000, 100, 16000, 100, 16000, 100, 100,
                    100, 16000, 100, 94375, 32, 132994, 32, 61462, 4, 72010, 178, 0, 1, 22151, 32,
                    91189, 769, 4, 2, 85848, 123203, 7305, -900, 1716, 549, 57, 85848, 0, 1, 1,
                    1000, 42921, 4, 2, 24548, 29498, 38, 1, 898148, 27279, 1, 51775, 558, 1, 39184,
                    1000, 60594, 1, 141895, 32, 83150, 32, 15299, 32, 76049, 1, 13169, 4, 22100,
                    10, 28999, 74, 1, 28999, 74, 1, 43285, 552, 1, 44749, 541, 1, 33852, 32, 68246,
                    32, 72362, 32, 7243, 32, 7391, 32, 11546, 32, 85848, 123203, 7305, -900, 1716,
                    549, 57, 85848, 0, 1, 90434, 519, 0, 1, 74433, 32, 85848, 123203, 7305, -900,
                    1716, 549, 57, 85848, 0, 1, 1, 85848, 123203, 7305, -900, 1716, 549, 57, 85848,
                    0, 1, 955506, 213312, 0, 2, 270652, 22588, 4, 1457325, 64566, 4, 20467, 1, 4,
                    0, 141992, 32, 100788, 420, 1, 1, 81663, 32, 59498, 32, 20142, 32, 24588, 32,
                    20744, 32, 25933, 32, 24623, 32, 43053543, 10, 53384111, 14333, 10, 43574283,
                    26308, 10, 16000, 100, 16000, 100, 962335, 18, 2780678, 6, 442008, 1, 52538055,
                    3756, 18, 267929, 18, 76433006, 8868, 18, 52948122, 18, 1995836, 36, 3227919,
                    12, 901022, 1, 166917843, 4307, 36, 284546, 36, 158221314, 26549, 36, 74698472,
                    36, 333849714, 1, 254006273, 72, 2174038, 72, 2261318, 64571, 4, 207616, 8310,
                    4, 1293828, 28716, 63, 0, 1, 1006041, 43623, 251, 0, 1, 100181, 726, 719, 0, 1,
                    100181, 726, 719, 0, 1, 100181, 726, 719, 0, 1, 107878, 680, 0, 1, 95336, 1,
                    281145, 18848, 0, 1, 180194, 159, 1, 1, 158519, 8942, 0, 1, 159378, 8813, 0, 1,
                    107490, 3298, 1, 106057, 655, 1, 1964219, 24520, 3,
                ]),
                unknown: BTreeMap::default(),
            },
            execution_costs: pallas_primitives::ExUnitPrices {
                mem_price: RationalNumber {
                    numerator: 577,
                    denominator: 10000,
                },
                step_price: RationalNumber {
                    numerator: 721,
                    denominator: 10000000,
                },
            },
            max_tx_ex_units: ExUnits {
                mem: 14000000,
                steps: 10000000000,
            },
            max_block_ex_units: ExUnits {
                mem: 62000000,
                steps: 40000000000,
            },
            max_value_size: 5000,
            collateral_percentage: 150,
            max_collateral_inputs: 3,
            pool_voting_thresholds: PoolVotingThresholds {
                motion_no_confidence: RationalNumber {
                    numerator: 0,
                    denominator: 1,
                },
                committee_normal: RationalNumber {
                    numerator: 0,
                    denominator: 1,
                },
                committee_no_confidence: RationalNumber {
                    numerator: 0,
                    denominator: 1,
                },
                hard_fork_initiation: RationalNumber {
                    numerator: 0,
                    denominator: 1,
                },
                security_voting_threshold: RationalNumber {
                    numerator: 0,
                    denominator: 1,
                },
            },
            drep_voting_thresholds: DRepVotingThresholds {
                motion_no_confidence: RationalNumber {
                    numerator: 0,
                    denominator: 1,
                },
                committee_normal: RationalNumber {
                    numerator: 0,
                    denominator: 1,
                },
                committee_no_confidence: RationalNumber {
                    numerator: 0,
                    denominator: 1,
                },
                update_constitution: RationalNumber {
                    numerator: 0,
                    denominator: 1,
                },
                hard_fork_initiation: RationalNumber {
                    numerator: 0,
                    denominator: 1,
                },
                pp_network_group: RationalNumber {
                    numerator: 0,
                    denominator: 1,
                },
                pp_economic_group: RationalNumber {
                    numerator: 0,
                    denominator: 1,
                },
                pp_technical_group: RationalNumber {
                    numerator: 0,
                    denominator: 1,
                },
                pp_governance_group: RationalNumber {
                    numerator: 0,
                    denominator: 1,
                },
                treasury_withdrawal: RationalNumber {
                    numerator: 0,
                    denominator: 1,
                },
            },
            min_committee_size: 0,
            committee_term_limit: 0,
            governance_action_validity_period: 0,
            governance_action_deposit: 0,
            drep_deposit: 0,
            drep_inactivity_period: 0,
            minfee_refscript_cost_per_byte: RationalNumber {
                numerator: 0,
                denominator: 1,
            },
        }
    }
}
