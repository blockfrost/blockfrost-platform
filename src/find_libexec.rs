use std::{
    env,
    path::{Path, PathBuf},
    process::Command,
};

use tracing::debug;

/// Searches for a “libexec” executable in multiple expected directories.
///
/// These are executables we use sort of like libraries, without linking them
/// into our executable. E.g. `hydra-node`, `testgen-hs`.
///
/// # Arguments
///
/// * `exe_name` - The name of the executable (without `.exe` on Windows).
///
/// * `env_name` - Allow overriding the path to the executable with this
///   environment variable name.
///
/// * `test_args` - Arguments to a test invocation of the found command (to
///   check that it really is executable). Maybe in the future we should have a
///   lambda to actually look at the output of this invocation?
///
pub fn find_libexec(exe_name: &str, env_name: &str, test_args: &[&str]) -> Result<String, String> {
    let env_var_dir: Option<PathBuf> = env::var(env_name)
        .ok()
        .and_then(|a| PathBuf::from(a).parent().map(|a| a.to_path_buf()));

    // This is the most important one for relocatable directories (that keep the initial
    // structure) on Windows, Linux, macOS:
    let current_exe_dir: Option<PathBuf> =
        std::fs::canonicalize(env::current_exe().map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?
            .parent()
            .map(|a| a.to_path_buf().join(exe_name));

    // Similar, but accounts for the `nix-bundle-exe` structure on Linux:
    let current_package_dir: Option<PathBuf> = current_exe_dir
        .clone()
        .and_then(|a| a.parent().map(PathBuf::from))
        .and_then(|a| a.parent().map(PathBuf::from));

    let cargo_target_dir: Option<PathBuf> = env::var("CARGO_MANIFEST_DIR")
        .ok()
        .map(|root| PathBuf::from(root).join("target/testgen-hs/extracted/testgen-hs"));

    let docker_path: Option<PathBuf> = Some(PathBuf::from(format!("/app/{exe_name}")));

    let system_path: Vec<PathBuf> = env::var("PATH")
        .map(|p| env::split_paths(&p).collect())
        .unwrap_or_default();

    let search_path: Vec<PathBuf> = vec![
        env_var_dir,
        current_exe_dir,
        current_package_dir,
        cargo_target_dir,
        docker_path,
    ]
    .into_iter()
    .flatten()
    .chain(system_path)
    .collect();

    let extension = if cfg!(target_os = "windows") {
        ".exe"
    } else {
        ""
    };

    let exe_name_ext = format!("{exe_name}{extension}");

    debug!("{} search directories = {:?}", exe_name_ext, search_path);

    // Checks if the path is runnable. Adjust for platform specifics if needed.
    // TODO: check that the --version matches what we expect.
    let is_our_executable =
        |path: &Path| -> bool { Command::new(path).args(test_args).output().is_ok() };

    // Look in each candidate directory to find a matching file
    for candidate in &search_path {
        let path = candidate.join(&exe_name_ext);

        if path.is_file() && is_our_executable(path.as_path()) {
            return Ok(path.to_string_lossy().to_string());
        }
    }

    Err(format!(
        "No valid `{}` binary found in {:?}.",
        exe_name_ext, &search_path
    ))
}
