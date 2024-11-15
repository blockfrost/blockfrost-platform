use crate::node::pool::NodePool;
use tokio::time::{self, Duration};
use tracing::error;

pub async fn node_health_check_task(node: NodePool) {
    loop {
        let health = node.get().await.map(drop).inspect_err(|err| {
            error!(
                "Health check: cannot get a working N2C connection from the pool: {:?}",
                err
            )
        });

        // Set delay based on health status
        let delay = Duration::from_secs(if health.is_ok() { 10 } else { 2 });

        time::sleep(delay).await;
    }
}
