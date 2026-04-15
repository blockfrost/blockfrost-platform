pub mod gateway;
pub mod platform;

use blockfrost::{BlockFrostSettings, BlockfrostAPI};
use std::sync::LazyLock;

static INIT_LOGGING: LazyLock<()> = LazyLock::new(|| {
    tracing_subscriber::fmt::init();
});

pub fn initialize_logging() {
    let _ = INIT_LOGGING;
}

pub fn blockfrost_preview_project_id() -> String {
    std::env::var("BLOCKFROST_PREVIEW_PROJECT_ID").expect(
        "BLOCKFROST_PREVIEW_PROJECT_ID is not set. \
         In CI it comes from a GitHub Secret. locally add it to .envrc.local.",
    )
}

pub fn get_blockfrost_client() -> BlockfrostAPI {
    let settings = BlockFrostSettings::default();

    BlockfrostAPI::new(&blockfrost_preview_project_id(), settings)
}
