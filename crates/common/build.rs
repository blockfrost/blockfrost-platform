use std::env;
use std::process::Command;

fn main() {
    const GIT_REVISION: &str = "GIT_REVISION";

    if env::var(GIT_REVISION).is_ok() {
        println!("cargo:warning=Environment variable {GIT_REVISION} is already set. Skipping.");
        return;
    }

    let git_status = Command::new("git")
        .args(["status", "--porcelain"])
        .output()
        .expect("git-status failed");

    let revision = if !git_status.stdout.is_empty() {
        "dirty".to_string()
    } else {
        let output = Command::new("git")
            .args(["rev-parse", "HEAD"])
            .output()
            .expect("git-rev-parse failed");

        String::from_utf8_lossy(&output.stdout).trim().to_string()
    };

    println!("cargo:rustc-env={}={}", GIT_REVISION, revision);
}
