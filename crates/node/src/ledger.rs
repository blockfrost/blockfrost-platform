use pallas_network::miniprotocols::localstate::{
    self,
    queries_v16::{CurrentProtocolParam, GenesisConfig},
};

use super::connection::NodeClient;

use bf_common::errors::BlockfrostError;

impl NodeClient {
    pub async fn genesis_config_and_pp(
        &mut self,
    ) -> Result<(GenesisConfig, CurrentProtocolParam, u64), BlockfrostError> {
        self.with_statequery(|generic_client: &mut localstate::GenericClient| {
            Box::pin(async {
                let era = localstate::queries_v16::get_current_era(generic_client).await?;
                let genesis =
                    localstate::queries_v16::get_genesis_config(generic_client, era).await?;
                let params =
                    localstate::queries_v16::get_current_pparams(generic_client, era).await?;
                let tip_slot = localstate::queries_v16::get_chain_point(generic_client)
                    .await?
                    .slot_or_default();
                Ok((genesis, params, tip_slot))
            })
        })
        .await
    }
}
