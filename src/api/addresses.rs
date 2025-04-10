use crate::{BlockfrostError, NodePool};
use axum::{Extension, Json, extract::Path, response::IntoResponse};
use pallas_network::miniprotocols::localstate::queries_v16;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
struct AddressUtxoContent {
    address: String,
    tx_hash: String,
    output_index: u64,
    amount: Vec<AddressUtxoContentAmount>,
    block: String,
    data_hash: Option<String>,
    inline_datum: Option<String>,
    reference_script_hash: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct AddressUtxoContentAmount {
    unit: String,
    quantity: String,
}

const UNIT_LOVELACE: &str = "lovelace";

/// See <https://docs.blockfrost.io/#tag/cardano--addresses/GET/addresses/{address}/utxos>, analogous to:
///
/// ```text
/// ❯ cardano-cli query utxo --testnet-magic 2 --address addr_test1qzv6t… --output-json
/// ```
pub async fn utxo_route(
    Path(addr): Path<String>,
    Extension(node): Extension<NodePool>,
) -> Result<impl IntoResponse, BlockfrostError> {
    // XXX: Axum must not abort Ouroboros protocols in the middle, hence a separate Tokio task:
    let utxos = tokio::spawn(async move {
        let mut node = node.get().await?;
        node.addresses_utxos(addr).await
    })
    .await
    .expect("addresses_utxos panic!")?;

    let mut response: Vec<AddressUtxoContent> = Vec::with_capacity(utxos.len());

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

        let amount: Vec<AddressUtxoContentAmount> = match amount {
            queries_v16::Value::Coin(coin) => vec![AddressUtxoContentAmount {
                unit: UNIT_LOVELACE.to_string(),
                quantity: Into::<u64>::into(coin).to_string(),
            }],
            queries_v16::Value::Multiasset(coin, multiasset) => {
                let mut rv = vec![AddressUtxoContentAmount {
                    unit: UNIT_LOVELACE.to_string(),
                    quantity: Into::<u64>::into(coin).to_string(),
                }];
                for (policy_id, assets) in multiasset.into_iter() {
                    for (asset_name, coin) in assets.into_iter() {
                        rv.push(AddressUtxoContentAmount {
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

        response.push(AddressUtxoContent {
            address: address.to_string(),
            tx_hash: utxo.transaction_id.to_string(),
            output_index: utxo.index.into(),
            amount,
            block,
            data_hash: data_hash.map(|a| a.to_string()),
            inline_datum,
            reference_script_hash,
        });
    }

    Ok(Json(response))
}
