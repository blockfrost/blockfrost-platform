use std::{
    env,
    path::{Path, PathBuf},
};

pub fn get_crate_root(crate_name: &str) -> PathBuf {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());

    Path::new(&manifest_dir).join("crates").join(crate_name)
}
