fn main() {
    let os = target_os();
    let arch = target_arch();

    bf_build_utils::git::set_git_env();
    bf_build_utils::testgen_hs::ensure(os, arch);
    bf_build_utils::features::evaluate(os, arch);
}

fn target_os() -> &'static str {
    (if cfg!(target_os = "macos") {
        "darwin"
    } else if cfg!(target_os = "linux") {
        "linux"
    } else if cfg!(target_os = "windows") {
        "windows"
    } else {
        panic!("Unsupported OS");
    }) as _
}

fn target_arch() -> &'static str {
    (if cfg!(target_arch = "x86_64") {
        "x86_64"
    } else if cfg!(target_arch = "aarch64") {
        "aarch64"
    } else {
        panic!("Unsupported architecture");
    }) as _
}