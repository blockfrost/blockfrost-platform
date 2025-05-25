use crate::{
    BlockfrostError, NodePool,
    addresses::{AddressInfo, AddressesPath},
    api::ApiResult,
    config::Config,
    pagination::{Pagination, PaginationQuery},
};
use axum::{
    Extension, Json,
    extract::{Path, Query},
};
use blockfrost_openapi::models::{
    address_utxo_content_inner::AddressUtxoContentInner,
    tx_content_output_amount_inner::TxContentOutputAmountInner,
};
use pallas_network::miniprotocols::localstate::{self, queries_v16};
use std::sync::Arc;

const UNIT_LOVELACE: &str = "lovelace";

/// See <https://docs.blockfrost.io/#tag/cardano--addresses/GET/addresses/{address}/utxos>, analogous to:
///
/// ```text
/// ❯ cardano-cli query utxo --testnet-magic 2 --address addr_test1qzv6t… --output-json
/// ```
pub async fn route(
    Path(address_path): Path<AddressesPath>,
    Extension(node): Extension<NodePool>,
    Extension(config): Extension<Arc<Config>>,
    Query(pagination_query): Query<PaginationQuery>,
) -> ApiResult<Vec<AddressUtxoContentInner>> {
    let AddressesPath { address, asset: _ } = address_path;
    let _ = Pagination::from_query(pagination_query).await?;
    let _ = AddressInfo::from_address(&address, config.network.clone())?;

    // XXX: Axum must not abort Ouroboros protocols in the middle, hence a separate Tokio task:
    let utxos = tokio::spawn(async move {
        let mut node = node.get().await?;

        let addr = pallas_addresses::Address::from_bech32(&address).map_err(|err| {
            BlockfrostError::custom_400(format!("invalid bech32 address: {}: {}", address, err))
        })?;

        node.with_statequery(|client: &mut localstate::GenericClient| {
            Box::pin(async move {
                let era: u16 = queries_v16::get_current_era(client).await?;
                let addrs: queries_v16::Addrs = Vec::from([addr.to_vec().into()]);
                let result = queries_v16::get_utxo_by_address(client, era, addrs)
                    .await?
                    .to_vec();
                Ok(result)
            })
        })
        .await
    })
    .await
    .expect("addresses_utxos panic!")?;

    let mut response: Vec<AddressUtxoContentInner> = Vec::with_capacity(utxos.len());

    for (utxo, tx_output) in utxos.into_iter() {
        let (address, amount, data_hash, inline_datum, reference_script) = match tx_output {
            queries_v16::TransactionOutput::Current(o) => {
                let (datum_hash, inline_datum) = match o.inline_datum {
                    Some(queries_v16::DatumOption::Hash(hash)) => (Some(hash), None),
                    Some(queries_v16::DatumOption::Data(data)) => (None, Some(data)),
                    None => (None, None),
                };
                (o.address, o.amount, datum_hash, inline_datum, o.script_ref)
            },
            queries_v16::TransactionOutput::Legacy(o) => {
                (o.address, o.amount, o.datum_hash, None, None)
            },
        };

        let address = pallas_addresses::Address::from_bytes(address.as_slice()).map_err(|err| {
            BlockfrostError::custom_400(format!("invalid bech32 addr: {}: {}", address, err))
        })?;

        let amount: Vec<TxContentOutputAmountInner> = match amount {
            queries_v16::Value::Coin(coin) => vec![TxContentOutputAmountInner {
                unit: UNIT_LOVELACE.to_string(),
                quantity: Into::<u64>::into(coin).to_string(),
            }],
            queries_v16::Value::Multiasset(coin, multiasset) => {
                let mut rv = vec![TxContentOutputAmountInner {
                    unit: UNIT_LOVELACE.to_string(),
                    quantity: Into::<u64>::into(coin).to_string(),
                }];
                for (policy_id, assets) in multiasset.into_iter() {
                    for (asset_name, coin) in assets.into_iter() {
                        rv.push(TxContentOutputAmountInner {
                            // FIXME: concatenated how? with a comma?
                            unit: format!("{},{}", policy_id, hex::encode(asset_name.as_slice())),
                            quantity: Into::<u64>::into(coin).to_string(),
                        });
                    }
                }
                rv
            },
        };

        let inline_datum = inline_datum.map(|id| hex::encode(minicbor::to_vec(id).unwrap())); // safe, infallible

        // FIXME: I think this is wrong, because it’s not the hash of the reference script, but the CBOR of the reference script itself?
        let reference_script_hash =
            reference_script.map(|id| hex::encode(minicbor::to_vec(id).unwrap())); // safe, infallible

        // TODO: The block hash may be impossible without a DB Sync (or similar)?
        let block = "impossible for now?".to_string();

        let output_index: u64 = utxo.index.into();

        response.push(AddressUtxoContentInner {
            address: address.to_string(),
            tx_hash: utxo.transaction_id.to_string(),
            tx_index: output_index as i32,
            output_index: output_index as i32,
            amount,
            block,
            data_hash: data_hash.map(|a| a.to_string()),
            inline_datum,
            reference_script_hash,
        });
    }

    Ok(Json(response))
}
