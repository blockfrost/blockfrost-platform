use std::borrow::Cow;
use std::collections::BTreeMap;
use std::str::FromStr;

use chrono::DateTime;
use chrono::NaiveDate;
use chrono::NaiveDateTime;
use chrono::NaiveTime;
use pallas_addresses::Address;
use pallas_codec::utils::AnyUInt;
use pallas_codec::utils::NonEmptyKeyValuePairs;
use pallas_network::miniprotocols::localstate::queries_v16;
use pallas_network::miniprotocols::localstate::queries_v16::CurrentProtocolParam;
use pallas_network::miniprotocols::localstate::queries_v16::GenesisConfig;
use pallas_network::miniprotocols::localstate::queries_v16::SystemStart;
use pallas_primitives::AssetName;
use pallas_primitives::Bytes;
use pallas_primitives::ExUnitPrices;
use pallas_primitives::ExUnits;
use pallas_primitives::PolicyId;
use pallas_primitives::PositiveCoin;
use pallas_primitives::RationalNumber;
use pallas_primitives::conway;
use pallas_traverse::MultiEraOutput;
use pallas_traverse::MultiEraTx;
use pallas_validate::utils::TxoRef;
use pallas_validate::{
    phase2::{EvalReport, script_context::SlotConfig},
    utils::{ConwayProtParams, MultiEraProtocolParameters, UtxoMap},
};

use crate::BlockfrostError;
use crate::NodePool;
use crate::api::utils::txs::evaluate::model::AdditionalUtxoSet;
use crate::api::utils::txs::evaluate::model::Value;
use crate::common::convert_bigint;
use pallas_codec::utils::CborWrap;
use pallas_primitives::{
    KeepRaw,
    conway::{DatumOption, ScriptRef},
};
use pallas_traverse::MultiEraInput;
use pallas_validate::utils::EraCbor;

//* This implementation uses pallas validate.
//  Since pallas validate behaves differently from the ogmios validation (which uses ledger)
//  this implementation is not used at the moment  */
pub async fn evaluate_binary_tx(
    node_pool: NodePool,
    tx_cbor_binary: &[u8],
    utxos: Option<AdditionalUtxoSet>,
) -> Result<EvalReport, BlockfrostError> {
    let slot_config = pallas_validate::phase2::script_context::SlotConfig::default();
    evaluate_tx(node_pool, tx_cbor_binary, &slot_config, utxos).await
}

pub async fn evaluate_encoded_tx(
    node_pool: NodePool,
    tx_cbor: &String,
    utxos: Option<AdditionalUtxoSet>,
) -> Result<EvalReport, BlockfrostError> {
    let tx_cbor_binary = hex::decode(tx_cbor).unwrap();

    evaluate_binary_tx(node_pool, &tx_cbor_binary, utxos).await
}

pub async fn evaluate_tx(
    node_pool: NodePool,
    tx_cbor_binary: &[u8],
    slot_config: &SlotConfig,
    utxo_set: Option<AdditionalUtxoSet>,
) -> Result<EvalReport, BlockfrostError> {
    let mut node = node_pool.get().await?;

    match node.genesis_config_and_pp().await {
        Ok((genesis_config, protocol_params)) => {
            let protocol_params: MultiEraProtocolParameters =
                convert_protocol_param(protocol_params, genesis_config)?;
            evaluate_tx_with_pp(tx_cbor_binary, slot_config, utxo_set, protocol_params)
        },
        Err(e) => Err(BlockfrostError::custom_400(
            "Error fetching protocol parameters: ".to_string() + &e.to_string(),
        )),
    }
}

pub fn evaluate_tx_with_pp(
    tx_cbor_binary: &[u8],
    slot_config: &SlotConfig,
    utxo_set: Option<AdditionalUtxoSet>,
    protocol_params: MultiEraProtocolParameters,
) -> Result<EvalReport, BlockfrostError> {
    /*
     * Prepare transaction
     */
    let multi_era_tx = MultiEraTx::decode(tx_cbor_binary).unwrap();

    //let multi_era_tx: MultiEraTx =  pallas_codec::minicbor::decode(tx_cbor_binary).unwrap();
    /*
     * make codec safe utxo for conway
     * with inputs, script and datum option
     */
    let mut utxos: UtxoMap = UtxoMap::new();
    for (tx_in, tx_out) in utxo_set.unwrap_or_default() {
        let alonzo_tx_in: conway::TransactionInput = conway::TransactionInput {
            transaction_id: pallas_primitives::Hash::<32>::from_str(&tx_in.tx_id)
                .expect("Invalid tx_id in additional utxo set"),
            index: tx_in.index,
        };

        let multi_era_in: MultiEraInput =
            MultiEraInput::AlonzoCompatible(Box::new(Cow::Owned(alonzo_tx_in)));

        /*
         * Prepare transaction output
         */

        let address = create_address(&tx_out.address);

        let value: pallas_primitives::conway::Value = convert_to_primitive_value(&tx_out.value);

        let datum_vec = convert_to_datum_option(&tx_out.datum);
        let datum_option = create_raw_datum_option(&datum_vec);

        let script_ref: Option<CborWrap<ScriptRef>> =
            tx_out.script.map(|script| CborWrap(script.into()));

        let post_alonzo = pallas_primitives::conway::PostAlonzoTransactionOutput {
            address,
            value,
            datum_option,
            script_ref,
        };

        //let tx_out_cbor  = pallas_codec::minicbor::to_vec(post_alonzo).unwrap();
        let tx_out_cbor = pallas_codec::minicbor::to_vec(post_alonzo).unwrap();
        //tx_out_cbors.push(tx_out_cbor.clone());
        let post_alonzo = pallas_codec::minicbor::decode::<
            pallas_codec::utils::KeepRaw<
                '_,
                pallas_primitives::conway::PostAlonzoTransactionOutput,
            >,
        >(&tx_out_cbor)
        .unwrap();
        let tx_out = pallas_primitives::conway::TransactionOutput::<'_>::PostAlonzo(post_alonzo);
        let multi_era_out = MultiEraOutput::<'_>::Conway(Box::new(Cow::Owned(tx_out)));

        utxos.insert(TxoRef::from(&multi_era_in), EraCbor::from(multi_era_out));
    }

    match pallas_validate::phase2::evaluate_tx(&multi_era_tx, &protocol_params, &utxos, slot_config)
    {
        Ok(report) => Ok(report),
        Err(e) => Err(BlockfrostError::custom_400(
            "Error evaluating transaction: ".to_string() + &e.to_string(),
        )),
    }
}

pub fn create_address(addr: &str) -> Bytes {
    Address::from_bech32(addr)
        .unwrap_or_else(|_| Address::from_hex(addr).unwrap())
        .to_vec()
        .into()
}

pub fn convert_to_datum_option(datum: &Option<String>) -> Vec<u8> {
    {
        match datum {
            Some(d) => {
                println!("Datum: {}", d);
                let datum_bytes = hex::decode(d).unwrap();
                let datum_option = DatumOption::Data(CborWrap(
                    pallas_codec::minicbor::decode(&datum_bytes).unwrap(),
                ));
                pallas_codec::minicbor::to_vec(datum_option).unwrap()
            },
            None => Vec::new(),
        }
    }
}

pub fn convert_to_datum_option_network(
    datum: &Option<String>,
) -> Option<pallas_network::miniprotocols::localstate::queries_v16::DatumOption> {
    {
        match datum {
            Some(d) => {
                println!("Datum: {}", d);
                let datum_bytes = hex::decode(d).unwrap();
                let datum_option =
                    pallas_network::miniprotocols::localstate::queries_v16::DatumOption::Data(
                        CborWrap(pallas_codec::minicbor::decode(&datum_bytes).unwrap()),
                    );
                Some(datum_option)
            },
            None => None,
        }
    }
}

pub fn create_raw_datum_option<'a>(datum_vec: &'a [u8]) -> Option<KeepRaw<'a, DatumOption<'a>>> {
    if datum_vec.is_empty() {
        None
    } else {
        let raw: KeepRaw<'a, DatumOption<'a>> = pallas_codec::minicbor::decode(datum_vec).unwrap();
        Some(raw)
    }
}

pub fn convert_to_primitive_value(value: &Value) -> pallas_primitives::conway::Value {
    match &value {
        Value {
            coins,
            assets: None,
        } => pallas_primitives::conway::Value::Coin(*coins),
        Value {
            coins,
            assets: Some(assets_map),
        } => {
            let mut assets = BTreeMap::new();
            for (id_name, number) in assets_map {
                let mut asset_detail = BTreeMap::new();
                let coin: PositiveCoin = number
                    .to_owned()
                    .try_into()
                    .expect("Invalid number for PositiveCoin in additional utxo output set");
                let (asset_id, asset_name) = parse_asset_string(id_name);

                asset_detail.insert(asset_name, coin);
                assets.insert(asset_id, asset_detail);
            }
            pallas_primitives::conway::Value::Multiasset(coins.to_owned(), assets)
        },
    }
}

pub fn convert_to_network_value(value: &Value) -> queries_v16::Value {
    match &value {
        Value {
            coins,
            assets: None,
        } => pallas_network::miniprotocols::localstate::queries_v16::Value::Coin(AnyUInt::U64(
            *coins,
        )),
        Value {
            coins,
            assets: Some(assets_map),
        } => {
            let mut assets = vec![];
            for (id_name, number) in assets_map {
                let mut asset_detail = vec![];
                let coin = AnyUInt::U64(*number);

                let (asset_id, asset_name) = parse_asset_string(id_name);

                asset_detail.push((asset_name, coin));
                assets.push((
                    asset_id,
                    NonEmptyKeyValuePairs::from_vec(asset_detail).unwrap(),
                ));
            }
            queries_v16::Value::Multiasset(
                AnyUInt::U64(*coins),
                NonEmptyKeyValuePairs::from_vec(assets).unwrap(),
            )
        },
    }
}

fn parse_asset_string(str: &str) -> (PolicyId, AssetName) {
    let mut parts = str.split('.');
    let policy_id = parts.next().expect("Invalid asset string in utxo output");
    let asset_name_bytes = hex::decode(parts.next().map(|s| s.to_string()).unwrap_or_default())
        .expect("Invalid asset name in asset string"); // can be empty
    let policy_id_bytes: [u8; 28] = hex::decode(policy_id)
        .expect("Invalid policy id in asset string")
        .try_into()
        .expect("Policy id is not valid in additional utxo output set");

    (
        PolicyId::from(policy_id_bytes),
        pallas_primitives::AssetName::from(asset_name_bytes),
    )
}

fn convert_system_start(
    sys_start: SystemStart,
) -> Result<chrono::DateTime<chrono::FixedOffset>, BlockfrostError> {
    let naive_date = NaiveDate::from_yo_opt(
        convert_bigint(sys_start.year)? as i32,
        sys_start.day_of_year as u32,
    )
    .expect("Invalid system start date");

    let picoseconds_of_day = convert_bigint(sys_start.picoseconds_of_day)?;
    let secs = (picoseconds_of_day / 1_000_000_000_000) as u32;
    let nano = ((picoseconds_of_day % 1_000_000_000_000) / 1000) as u32;

    let naive_time =
        NaiveTime::from_num_seconds_from_midnight_opt(secs, nano).expect("Invalid time");

    let naive_date_time = NaiveDateTime::new(naive_date, naive_time);

    // Convert to DateTime with UTC timezone (zero offset)
    let utc_offset = chrono::FixedOffset::east_opt(0).expect("Invalid offset");

    Ok(DateTime::from_naive_utc_and_offset(
        naive_date_time,
        utc_offset,
    ))
}

fn convert_protocol_param(
    pp: CurrentProtocolParam,
    genesis: GenesisConfig,
) -> Result<MultiEraProtocolParameters, BlockfrostError> {
    // Create a Conway protocol parameters struct from the ProtocolParam and GenesisConfig
    let conway_pp: ConwayProtParams = ConwayProtParams {
        minfee_a: pp.minfee_a.unwrap() as u32,
        minfee_b: pp.minfee_b.unwrap() as u32,
        max_block_body_size: pp.max_block_body_size.unwrap() as u32,
        max_transaction_size: pp.max_transaction_size.unwrap() as u32,
        max_block_header_size: pp.max_block_header_size.unwrap() as u32,
        system_start: convert_system_start(genesis.system_start)?,
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

    Ok(MultiEraProtocolParameters::Conway(conway_pp))
}

#[cfg(test)]
mod tests {

    use chrono::{Datelike, Timelike};

    use pallas_codec::utils::AnyUInt;
    use pallas_network::miniprotocols::localstate::queries_v16::BigInt;
    use pallas_network::miniprotocols::localstate::queries_v16::{self, Fraction};

    use super::*;

    #[test]
    fn test_convert_system_start() {
        // Test case 1: Standard date
        let sys_start = SystemStart {
            year: BigInt::from(2023),
            day_of_year: 100,
            picoseconds_of_day: BigInt::from(43_200_000_000_000_000i64), // 12 hours
        };
        let result = convert_system_start(sys_start).unwrap();
        assert_eq!(result.year(), 2023);
        assert_eq!(result.ordinal(), 100);
        assert_eq!(result.hour(), 12);
        assert_eq!(result.minute(), 0);
        assert_eq!(result.second(), 0);

        // Test case 2: Year boundary
        let sys_start = SystemStart {
            year: BigInt::from(2024),
            day_of_year: 1,
            picoseconds_of_day: BigInt::from(3_600_000_000_000_000i64), // 1 hour
        };
        let result = convert_system_start(sys_start).unwrap();
        assert_eq!(result.year(), 2024);
        assert_eq!(result.ordinal(), 1);
        assert_eq!(result.hour(), 1);
        assert_eq!(result.minute(), 0);
        assert_eq!(result.second(), 0);

        // Test case 3: Leap year
        let sys_start = SystemStart {
            year: BigInt::from(2024),
            day_of_year: 366,
            picoseconds_of_day: BigInt::from(0), // midnight
        };
        let result = convert_system_start(sys_start).unwrap();
        assert_eq!(result.year(), 2024);
        assert_eq!(result.ordinal(), 366);
        assert_eq!(result.hour(), 0);
        assert_eq!(result.minute(), 0);
        assert_eq!(result.second(), 0);

        // Test case 4: Partial seconds
        let sys_start = SystemStart {
            year: BigInt::from(2023),
            day_of_year: 200,
            picoseconds_of_day: BigInt::from(63_123_000_000_000_000i64), // 17:32:03
        };
        let result = convert_system_start(sys_start).unwrap();
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
                year: BigInt::from(2023),
                day_of_year: 100,
                picoseconds_of_day: BigInt::from(43_200_000_000_000_000i64), // 12 hours
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
        let pp = CurrentProtocolParam {
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
            protocol_version: Some((10, 0)),
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
        let result = convert_protocol_param(pp, genesis).unwrap();

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
        }
    }
}
