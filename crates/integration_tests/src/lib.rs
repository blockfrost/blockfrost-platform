pub mod platform;

use blockfrost::{BlockFrostSettings, BlockfrostAPI};
use std::sync::LazyLock;

static INIT_LOGGING: LazyLock<()> = LazyLock::new(|| {
    tracing_subscriber::fmt::init();
});

pub fn initialize_logging() {
    let _ = INIT_LOGGING;
}

pub fn get_blockfrost_client() -> BlockfrostAPI {
    let settings = BlockFrostSettings::default();

    BlockfrostAPI::new("previewy2pbyga8FifUwJSverBCwhESegV6I7gT", settings)
}
