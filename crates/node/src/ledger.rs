use pallas_network::miniprotocols::localstate::{
    self,
    queries_v16::{CurrentProtocolParam, GenesisConfig},
};

use super::connection::NodeClient;

use bf_common::errors::BlockfrostError;

impl NodeClient {
    /// Fetches the current protocol parameters from the connected Cardano node.
    /// @TODO These values can be cached
    pub async fn protocol_params(&mut self) -> Result<CurrentProtocolParam, BlockfrostError> {
        self.with_statequery(|generic_client: &mut localstate::GenericClient| {
            Box::pin(async {
                let era = localstate::queries_v16::get_current_era(generic_client).await?;
                let params =
                    localstate::queries_v16::get_current_pparams(generic_client, era).await?;
                Ok(params)
            })
        })
        .await
    }

    /// @TODO These values can be cached or read from a genesis file
    pub async fn genesis_config(&mut self) -> Result<GenesisConfig, BlockfrostError> {
        self.with_statequery(|generic_client: &mut localstate::GenericClient| {
            Box::pin(async {
                let era = localstate::queries_v16::get_current_era(generic_client).await?;
                let genesis =
                    localstate::queries_v16::get_genesis_config(generic_client, era).await?;
                Ok(genesis)
            })
        })
        .await
    }

    pub async fn genesis_config_and_pp(
        &mut self,
    ) -> Result<(GenesisConfig, CurrentProtocolParam), BlockfrostError> {
        self.with_statequery(|generic_client: &mut localstate::GenericClient| {
            Box::pin(async {
                let era = localstate::queries_v16::get_current_era(generic_client).await?;
                let genesis =
                    localstate::queries_v16::get_genesis_config(generic_client, era).await?;
                let params =
                    localstate::queries_v16::get_current_pparams(generic_client, era).await?;
                Ok((genesis, params))
            })
        })
        .await
    }
}
