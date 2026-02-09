use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "blockfrost-sdk-bridge", // otherwise it’s `blockfrost-gateway`
          bin_name = "blockfrost-sdk-bridge",
          version, about, long_about = None)]
struct Cli {
    /// A prefunded L1 key file for paying the Hydra transaction fees on L1, ~13 ADA per L2 cycle.
    #[arg(long)]
    pub hydra_cardano_signing_key: PathBuf,
}

fn main() {
    let cli = Cli::parse();
    println!("Hello, {:?}!", cli.hydra_cardano_signing_key);
}
