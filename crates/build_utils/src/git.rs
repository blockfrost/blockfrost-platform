use std::env;
use std::process::Command;
const GIT_REVISION: &str = "GIT_REVISION";

pub fn set_git_env() {
    if env::var(GIT_REVISION).is_ok() {
        println!("Environment variable {GIT_REVISION} is set. Not setting.");
        return;
    }

    let git_status = Command::new("git")
        .args(["status", "--porcelain"])
        .output()
        .expect("git-status");

    let revision = if !git_status.stdout.is_empty() {
        "dirty".to_string()
    } else {
        let git_rev_parse = Command::new("git")
            .args(["rev-parse", "HEAD"])
            .output()
            .expect("git-rev-parse");

        String::from_utf8_lossy(&git_rev_parse.stdout)
            .trim()
            .to_string()
    };

    println!("cargo:rustc-env={GIT_REVISION}={revision}");
}
