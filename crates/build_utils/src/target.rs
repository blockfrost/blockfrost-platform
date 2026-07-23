use std::{env, path::PathBuf};

pub fn os() -> String {
    match env::var("CARGO_CFG_TARGET_OS")
        .expect("Unable to find target OS")
        .as_str()
    {
        "macos" => "darwin".into(),
        "linux" => "linux".into(),
        "windows" => "windows".into(),
        os => panic!("Unsupported OS: {os}"),
    }
}

pub fn arch() -> String {
    match env::var("CARGO_CFG_TARGET_ARCH")
        .expect("Unable to find target architecture")
        .as_str()
    {
        "x86_64" => "x86_64".into(),
        "aarch64" => "aarch64".into(),
        arch => panic!("Unsupported architecture: {arch}"),
    }
}

/// Returns Cargo's actual profile output directory (`target/debug`,
/// `target/release`, or the equivalent beneath a custom target directory).
pub fn profile_dir() -> PathBuf {
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("Unable to find OUT_DIR"));

    // OUT_DIR is `<target>/<profile>/build/<crate>-<hash>/out`; this also works
    // when `<target>` includes a cross-compilation target triple.
    out_dir
        .ancestors()
        .nth(3)
        .expect("OUT_DIR has an unexpected structure")
        .to_path_buf()
}
