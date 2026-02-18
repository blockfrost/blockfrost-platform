use crate::sync_progress::NodeInfo;
use bf_common::errors::BlockfrostError;
use std::sync::Arc;
use tokio::sync::Mutex;

const CHAIN_STALE_IF_OLDER_THAN: std::time::Duration = std::time::Duration::from_secs(5 * 60);

pub struct ChainStalenessMonitor {
    last_chain_advancement: std::time::Instant,
    last_chain_block: String,
    errors: Arc<Mutex<Vec<BlockfrostError>>>,
}

impl Default for ChainStalenessMonitor {
    fn default() -> Self {
        Self::new()
    }
}

impl ChainStalenessMonitor {
    pub fn new() -> Self {
        Self {
            last_chain_advancement: std::time::Instant::now(),
            last_chain_block:
                "0000000000000000000000000000000000000000000000000000000000000000".to_string(),
            errors: Arc::new(Mutex::new(vec![])),
        }
    }

    pub async fn update(&mut self, node_info: &Option<NodeInfo>) {
        if let Some(node_info) = node_info {
            if self.last_chain_block != node_info.block {
                self.last_chain_block = node_info.block.clone();
                self.last_chain_advancement = std::time::Instant::now();
            }
        }

        let elapsed = self.last_chain_advancement.elapsed();

        *(self.errors.lock().await) = if elapsed > CHAIN_STALE_IF_OLDER_THAN {
            let err = format!(
                "Chain stuck at {}, has not seen updates in {:?}.",
                self.last_chain_block, elapsed
            );
            tracing::error!("{}", err);
            vec![BlockfrostError::internal_server_error(err)]
        } else {
            vec![]
        };
    }

    pub fn errors(&self) -> Arc<Mutex<Vec<BlockfrostError>>> {
        self.errors.clone()
    }
}
