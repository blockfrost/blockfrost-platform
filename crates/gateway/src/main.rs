#[cfg(not(windows))]
fn main() -> anyhow::Result<()> {
    blockfrost_gateway::gateway_main::run()
}

#[cfg(windows)]
fn main() {
    eprintln!("blockfrost-gateway is not supported on Windows");
}
