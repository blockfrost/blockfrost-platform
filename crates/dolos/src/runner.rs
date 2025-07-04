use crate::config::Config;
use common::config::Config as RootConfig;
use common::errors::AppError;
use std::env;
use std::process::Command;

pub async fn run_dolos_daemon(root_config: RootConfig) -> Result<_, AppError> {
    // Take root configuration and generate configuration for dolos
    let config = Config::generate_from_root_config(&root_config).await?;

    // Saves configuration to toml file
    config.save_to_toml("../dolos.toml").await?;

    let dolos_bin = env::var("DOLOS_BIN").unwrap_or_else(|_| "dolos".to_string());
    let args = vec![
        "daemon".to_string(),
        "--config".to_string(),
        "../dolos.toml".to_string(),
    ];

    let mut child = Command::new(dolos_bin).args(&args).spawn().unwrap();

    child.wait().unwrap();
}
