use std::{env, fs, path::PathBuf, process::Command};
use toml;

/// Reads the dolos version from `package.metadata.dolos.version`
fn get_dolos_version() -> String {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR")
        .expect("build.rs: CARGO_MANIFEST_DIR environment variable is not set");
    let cargo_toml_path = PathBuf::from(&manifest_dir).join("Cargo.toml");
    let cargo_toml = fs::read_to_string(&cargo_toml_path).unwrap_or_else(|e| {
        panic!(
            "build.rs: failed to read {}: {}",
            cargo_toml_path.display(),
            e
        )
    });

    let value: toml::Value = cargo_toml
        .parse()
        .expect("build.rs: failed to parse Cargo.toml as TOML");

    value["package"]["metadata"]["dolos"]["version"]
        .as_str()
        .expect("build.rs: `package.metadata.dolos.version` key missing or not a string")
        .to_owned()
}

/// Downloads the dolos binary into `<workspace-root>/target/dolos/bin/<version>/dolos`
/// if it doesn't already exist, then exports its path via `cargo:rustc-env=DOLOS_BIN_PATH=…`
pub fn fetch_binary() {
    const ENV_KEY: &str = "DOLOS_BIN_PATH";
    let version = get_dolos_version();

    // Determine the true target directory:
    // - Use CARGO_TARGET_DIR if set
    // - Otherwise, assume `<workspace-root>/target`
    let target_dir: PathBuf = match env::var("CARGO_TARGET_DIR") {
        Ok(dir) => dir.into(),
        Err(_) => {
            let manifest_dir =
                env::var("CARGO_MANIFEST_DIR").expect("build.rs: CARGO_MANIFEST_DIR not set");
            PathBuf::from(manifest_dir)
                .parent()
                .expect("build.rs: expected `crates/` directory")
                .parent()
                .expect("build.rs: expected workspace root")
                .join("target")
        },
    };

    let downloaded_dir = target_dir.join("dolos").join("bin").join(&version);
    let exe_name = if cfg!(windows) { "dolos.exe" } else { "dolos" };
    let dolos_bin_path = downloaded_dir.join(exe_name);

    if !downloaded_dir.exists() {
        fs::create_dir_all(&downloaded_dir).unwrap_or_else(|e| {
            panic!(
                "build.rs: failed to create {}: {}",
                downloaded_dir.display(),
                e
            )
        });
    }

    // Download
    if !dolos_bin_path.exists() {
        println!("Downloading dolos version {version}…");

        let script_path = PathBuf::from(
            env::var("CARGO_MANIFEST_DIR").expect("build.rs: CARGO_MANIFEST_DIR not set"),
        )
        .join("scripts")
        .join("download.sh");

        let status = Command::new("bash")
            .arg(&script_path)
            .arg(&version)
            .arg(&downloaded_dir)
            .status()
            .unwrap_or_else(|e| {
                panic!(
                    "build.rs: failed to spawn download script `{}`: {}",
                    script_path.display(),
                    e
                )
            });

        if !status.success() {
            panic!(
                "build.rs: download script `{}` exited with status {}",
                script_path.display(),
                status
            );
        }
    }

    // Verify TODO it's not working with nix
    // let output = Command::new(&dolos_bin_path)
    //     .arg("--version")
    //     .output()
    //     .unwrap_or_else(|e| {
    //         panic!(
    //             "build.rs: failed to execute downloaded binary `{}`: {}",
    //             dolos_bin_path.display(),
    //             e
    //         )
    //     });

    // if !output.status.success() {
    //     let stderr = String::from_utf8_lossy(&output.stderr);
    //     panic!(
    //         "build.rs: downloaded binary `{}` failed `--version`: {}",
    //         dolos_bin_path.display(),
    //         stderr
    //     );
    // }

    // Export
    println!("cargo:rustc-env={}={}", ENV_KEY, dolos_bin_path.display());
}
