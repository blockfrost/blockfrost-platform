use super::connection::NodeClient;
use crate::BlockfrostError;
use chrono::{Duration, TimeZone, Utc};
use pallas_network::{miniprotocols, miniprotocols::localstate};
use pallas_traverse::wellknown;
use serde::{Deserialize, Serialize};
use std::boxed::Box;

#[derive(Serialize, Deserialize, Debug)]
pub struct NodeInfo {
    pub block: String,
    pub epoch: u32,
    pub era: String,
    pub slot: u64,
    pub sync_progress: f64,
}

impl NodeClient {
    /// Reports the sync progress of the node.
    pub async fn sync_progress(&mut self) -> Result<NodeInfo, BlockfrostError> {
        self.with_statequery(|generic_client: &mut localstate::GenericClient| {
            Box::pin(async {
                let current_era = localstate::queries_v16::get_current_era(generic_client).await?;

                let epoch =
                    localstate::queries_v16::get_block_epoch_number(generic_client, current_era)
                        .await?;

                let geneses =
                    localstate::queries_v16::get_genesis_config(generic_client, current_era)
                        .await?;
                let genesis = geneses.first().ok_or_else(|| {
                    BlockfrostError::internal_server_error(
                        "Expected at least one genesis".to_string(),
                    )
                })?;

                let system_start =
                    localstate::queries_v16::get_system_start(generic_client).await?;
                let chain_point = localstate::queries_v16::get_chain_point(generic_client).await?;
                let slot = chain_point.slot_or_default();

                // FIXME: this is debatable, because it won’t work for custom networks; we should rather
                // get this information by calling `Ouroboros.Consensus.HardFork.History.Qry.slotToWallclock`
                // like both cardano-cli (through cardano-api) and Ogmios do, but it’s not implemented
                // in pallas_network yet.
                let wellknown_genesis = wellknown::GenesisValues::from_magic(
                    genesis.network_magic.into(),
                )
                .ok_or_else(|| {
                    BlockfrostError::internal_server_error(format!(
                        "Only well-known networks are supported (unsupported network magic: {})",
                        genesis.network_magic
                    ))
                })?;

                let year: i32 = system_start.year.try_into().map_err(|e| {
                    BlockfrostError::internal_server_error(format!("Failed to convert year: {}", e))
                })?;

                let base_date = Utc
                    .with_ymd_and_hms(year, 1, 1, 0, 0, 0)
                    .single()
                    .ok_or_else(|| {
                        BlockfrostError::internal_server_error("Invalid base date".to_string())
                    })?;

                let days = Duration::days((system_start.day_of_year - 1).into());

                let nanoseconds: i64 = (system_start.picoseconds_of_day / 1_000)
                    .try_into()
                    .map_err(|e| {
                        BlockfrostError::internal_server_error(format!(
                            "Failed to convert picoseconds: {}",
                            e
                        ))
                    })?;

                let duration_ns = Duration::nanoseconds(nanoseconds);

                let utc_start = base_date + days + duration_ns;

                let slot_time_secs: i64 = wellknown_genesis
                    .slot_to_wallclock(slot)
                    .try_into()
                    .map_err(|e| {
                        BlockfrostError::internal_server_error(format!(
                            "Failed to convert slot time: {}",
                            e
                        ))
                    })?;

                let utc_slot = Utc
                    .timestamp_opt(slot_time_secs, 0)
                    .single()
                    .ok_or_else(|| {
                        BlockfrostError::internal_server_error("Invalid slot timestamp".to_string())
                    })?;

                let utc_now = Utc::now();

                let utc_slot_capped = std::cmp::min(utc_now, utc_slot);

                let tolerance = 60; // [s]
                let percentage = if (utc_now - utc_slot_capped).num_seconds() < tolerance {
                    100.00
                } else {
                    let network_duration = (utc_now - utc_start).num_seconds() as f64;
                    let duration_up_to_slot = (utc_slot_capped - utc_start).num_seconds() as f64;
                    // Multiply by 100 to get a percentage, then multiply by 100 again, round, and divide by 100
                    // to limit the result to two decimal places.
                    ((duration_up_to_slot / network_duration) * 100.0 * 100.0).round() / 100.0
                };

                let block = match chain_point {
                    miniprotocols::Point::Origin => String::new(),
                    miniprotocols::Point::Specific(_, block) => hex::encode(&block),
                };

                Ok(NodeInfo {
                    sync_progress: percentage,
                    era: current_era.to_string(),
                    epoch,
                    slot,
                    block,
                })
            })
        })
        .await
    }
}
