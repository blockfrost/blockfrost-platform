#[cfg(not(windows))]
fn main() -> anyhow::Result<()> {
    blockfrost_gateway::bridge_main::run()
}

#[cfg(windows)]
fn main() {
    eprintln!("blockfrost-sdk-bridge is not supported on Windows");
}
