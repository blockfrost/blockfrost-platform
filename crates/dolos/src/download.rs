use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};
use toml;

fn get_crate_root() -> PathBuf {
    let cargo_manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");

    Path::new(&cargo_manifest_dir).join("crates").join("dolos")
}

fn get_dolos_version() -> String {
    let crate_root = get_crate_root();
    let cargo_toml_path = crate_root.join("Cargo.toml");
    let cargo_toml = fs::read_to_string(&cargo_toml_path)
        .unwrap_or_else(|_| panic!("Failed to read {}", cargo_toml_path.display()));

    let value: toml::Value = cargo_toml.parse().expect("Invalid Cargo.toml");

    value["package"]["metadata"]["dolos"]["version"]
        .as_str()
        .expect("version missing")
        .to_string()
}

pub fn resolve() {
    const DOLOS_BIN_PATH: &str = "DOLOS_BIN_PATH";

    let crate_root = get_crate_root();
    let dolos_version = get_dolos_version();
    let downloaded_dir = Path::new(&crate_root).join("bin").join(&dolos_version);
    let dolos_bin_path = downloaded_dir.join("dolos");

    if dolos_bin_path.exists() {
        println!("dolos already present at {}", dolos_bin_path.display());
    } else {
        println!("Downloading dolos version {}...", dolos_version);
        let script_path = crate_root.join("scripts").join("download.sh");

        let status = Command::new("bash")
            .arg(&script_path)
            .arg(&dolos_version)
            .status()
            .unwrap_or_else(|_| panic!("Failed to run {}", script_path.display()));

        if !status.success() {
            panic!("download.sh failed with status: {}", status);
        }
    }

    let version_output = Command::new(&dolos_bin_path)
        .arg("--version")
        .output()
        .expect("dolos --version failed");

    println!("version_output {:?}", version_output);

    if !version_output.status.success() {
        panic!(
            "Failed to run dolos --version: {}",
            String::from_utf8_lossy(&version_output.stderr)
        );
    }

    let version = String::from_utf8_lossy(&version_output.stdout)
        .trim()
        .to_string();

    println!(
        "cargo:rustc-env={}={}",
        DOLOS_BIN_PATH,
        dolos_bin_path.display()
    );

    println!("cargo:rustc-env={}={}", version, version);
}
