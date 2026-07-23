use std::{
    env,
    path::{Path, PathBuf},
};

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

/// Ensures every directory under `root` (inclusive) is writable by the owner.
///
/// Archives extracted from the Nix store keep read-only (`0500`) directory
/// modes. Without a writable bit on a directory, its entries can't be removed,
/// which breaks re-extraction on a subsequent build.
fn make_dirs_writable(root: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::{fs::set_permissions, os::unix::fs::PermissionsExt};

        for dir in walk_dirs(root)? {
            let mut perms = dir.metadata()?.permissions();
            let mode = perms.mode();
            if mode & 0o200 == 0 {
                perms.set_mode(mode | 0o700);
                set_permissions(&dir, perms)?;
            }
        }
    }
    #[cfg(not(unix))]
    let _ = root;
    Ok(())
}

/// Like [`std::fs::remove_dir_all`], but first makes all directories writable so
/// read-only extraction trees (see [`make_dirs_writable`]) can be removed.
pub fn remove_dir_all_writable(root: &Path) -> std::io::Result<()> {
    make_dirs_writable(root)?;
    std::fs::remove_dir_all(root)
}

/// Collects `root` and all of its descendant directories so that a caller may
/// adjust their permissions.
fn walk_dirs(root: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut dirs = vec![root.to_path_buf()];
    let mut i = 0;
    while i < dirs.len() {
        let current = dirs[i].clone();
        i += 1;
        // A symlink's `metadata()` follows the link; only recurse into real dirs.
        if current.symlink_metadata()?.file_type().is_symlink() {
            continue;
        }
        for entry in std::fs::read_dir(&current)? {
            let path = entry?.path();
            if path.symlink_metadata()?.file_type().is_dir() {
                dirs.push(path);
            }
        }
    }
    Ok(dirs)
}
