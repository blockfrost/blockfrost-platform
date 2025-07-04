use crate::config::Config;
use common::{config::Config as RootConfig, errors::AppError};
// use std::{env, process::Command};

pub async fn run_daemon(root_config: RootConfig) -> Result<(), AppError> {
    let config_path = "../dolos.toml";
    let config = Config::generate_from_root_config(&root_config).await?;

    config.save_to_toml(config_path)?;

    Ok(())
    // let dolos_bin = env::var("DOLOS_BIN").unwrap_or_else(|_| "dolos".to_string());
    // let args = vec![
    //     "daemon".to_string(),
    //     "--config".to_string(),
    //     config_path.to_string(),
    // ];

    // let mut child = Command::new(dolos_bin).args(&args).spawn().unwrap();

    // child.wait().unwrap();
}
