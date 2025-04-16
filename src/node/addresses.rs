use super::connection::NodeClient;
use crate::BlockfrostError;
use pallas_network::miniprotocols::localstate::{self, queries_v16};

impl NodeClient {
    pub async fn addresses_utxos(
        &mut self,
        addr: String,
    ) -> Result<Vec<(queries_v16::UTxO, queries_v16::TransactionOutput)>, BlockfrostError> {
        let addr = pallas_addresses::Address::from_bech32(&addr).map_err(|err| {
            BlockfrostError::custom_400(format!("invalid bech32 address: {}: {}", addr, err))
        })?;

        self.with_statequery(|client: &mut localstate::GenericClient| {
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
    }
}
