use pallas_network::miniprotocols::localstate::{
    self,
    queries_v16::{TxIns, UTxOByTxin},
};

use super::connection::NodeClient;

use crate::BlockfrostError;

impl NodeClient {
    pub async fn get_utxos_for_txins(&mut self, ins: TxIns) -> Result<UTxOByTxin, BlockfrostError> {
        self.with_statequery(|generic_client: &mut localstate::GenericClient| {
            Box::pin(async {
                let era = localstate::queries_v16::get_current_era(generic_client).await?;

                let utxos: UTxOByTxin =
                    localstate::queries_v16::get_utxo_by_txin(generic_client, era, ins).await?;
                Ok(utxos)
            })
        })
        .await
    }
}
