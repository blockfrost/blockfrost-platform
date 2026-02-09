#[cfg(not(windows))]
fn main() {
    git_revision::set();
    hydra_scripts_id::set();
}

#[cfg(windows)]
fn main() {}

#[cfg(not(windows))]
mod git_revision {
    use std::env;

    const GIT_REVISION: &str = "GIT_REVISION";

    pub fn set() {
        use std::process::Command;

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
}

#[cfg(not(windows))]
mod hydra_scripts_id {
    use std::{
        collections::HashMap,
        env, fs,
        path::{Path, PathBuf},
        time::Duration,
    };

    pub fn set() {
        println!("cargo:rerun-if-env-changed=HYDRA_SCRIPTS_TX_ID_MAINNET");
        println!("cargo:rerun-if-env-changed=HYDRA_SCRIPTS_TX_ID_PREPROD");
        println!("cargo:rerun-if-env-changed=HYDRA_SCRIPTS_TX_ID_PREVIEW");

        // If user already provided the values at build time, honor them and avoid network.
        if let (Ok(m), Ok(p), Ok(v)) = (
            env::var("HYDRA_SCRIPTS_TX_ID_MAINNET"),
            env::var("HYDRA_SCRIPTS_TX_ID_PREPROD"),
            env::var("HYDRA_SCRIPTS_TX_ID_PREVIEW"),
        ) {
            set_envs(&m, &p, &v, None, None);
            return;
        }

        let manifest_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
        let flake_lock = find_upwards(&manifest_dir, "flake.lock").unwrap_or_else(|| {
            panic!(
                "Could not find flake.lock by walking up from {}",
                manifest_dir.display()
            )
        });

        println!("cargo:rerun-if-changed={}", flake_lock.display());

        let flake_lock_json = fs::read_to_string(&flake_lock)
            .unwrap_or_else(|e| panic!("Failed to read {}: {e}", flake_lock.display()));

        let (rev, href) = read_hydra_rev_and_ref(&flake_lock_json);

        let url = format!(
            "https://raw.githubusercontent.com/cardano-scaling/hydra/{rev}/hydra-node/networks.json"
        );

        let networks_json = fetch_cached(&url, &rev);
        let networks_map: HashMap<String, HashMap<String, String>> =
            serde_json::from_str(&networks_json).unwrap_or_else(|e| {
                panic!("Failed to parse networks.json downloaded from {url}: {e}")
            });

        let mainnet = lookup(&networks_map, "mainnet", &href);
        let preprod = lookup(&networks_map, "preprod", &href);
        let preview = lookup(&networks_map, "preview", &href);

        set_envs(&mainnet, &preprod, &preview, Some(&rev), Some(&href));
    }

    fn set_envs(
        mainnet: &str,
        preprod: &str,
        preview: &str,
        rev: Option<&str>,
        href: Option<&str>,
    ) {
        // These are what your env!("...") will see.
        println!("cargo:rustc-env=HYDRA_SCRIPTS_TX_ID_MAINNET={mainnet}");
        println!("cargo:rustc-env=HYDRA_SCRIPTS_TX_ID_PREPROD={preprod}");
        println!("cargo:rustc-env=HYDRA_SCRIPTS_TX_ID_PREVIEW={preview}");

        // Extra metadata (optional, but often handy)
        if let Some(rev) = rev {
            println!("cargo:rustc-env=HYDRA_INPUT_REV={rev}");
        }
        if let Some(href) = href {
            println!("cargo:rustc-env=HYDRA_INPUT_REF={href}");
        }
    }

    fn read_hydra_rev_and_ref(flake_lock_json: &str) -> (String, String) {
        let v: serde_json::Value = serde_json::from_str(flake_lock_json)
            .unwrap_or_else(|e| panic!("Failed to parse flake.lock JSON: {e}"));

        let rev = v
            .pointer("/nodes/hydra/locked/rev")
            .and_then(|x| x.as_str())
            .unwrap_or_else(|| panic!("flake.lock missing /nodes/hydra/locked/rev"))
            .to_string();

        let href = v
            .pointer("/nodes/hydra/original/ref")
            .and_then(|x| x.as_str())
            .unwrap_or_else(|| panic!("flake.lock missing /nodes/hydra/original/ref"))
            .to_string();

        (rev, href)
    }

    fn lookup(
        networks: &HashMap<String, HashMap<String, String>>,
        network: &str,
        href: &str,
    ) -> String {
        networks
            .get(network)
            .unwrap_or_else(|| panic!("networks.json missing top-level key {network:?}"))
            .get(href)
            .cloned()
            .unwrap_or_else(|| {
                let mut versions: Vec<_> = networks[network].keys().cloned().collect();
                versions.sort();
                panic!(
                    "networks.json has no entry for network {network:?} version/ref {href:?}. \
Available versions: {}",
                    versions.join(", ")
                )
            })
    }

    fn fetch_cached(url: &str, rev: &str) -> String {
        let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());
        let cache_path = out_dir.join(format!("hydra-networks.{rev}.json"));

        // If it's already in OUT_DIR, reuse it (build scripts can run a lot).
        if let Ok(s) = fs::read_to_string(&cache_path) {
            return s;
        }

        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(20))
            .user_agent("cargo-build-script (hydra networks.json fetch)")
            .build()
            .expect("Failed to build reqwest client");

        let resp = client
            .get(url)
            .send()
            .unwrap_or_else(|e| panic!("Failed to GET {url}: {e}"));

        if !resp.status().is_success() {
            panic!("GET {url} failed with status {}", resp.status());
        }

        let text = resp
            .text()
            .unwrap_or_else(|e| panic!("Failed to read response body from {url}: {e}"));

        // Best-effort cache; ignore failures.
        let _ = fs::write(&cache_path, &text);

        text
    }

    fn find_upwards(start: &Path, file_name: &str) -> Option<PathBuf> {
        let mut dir = Some(start);

        while let Some(d) = dir {
            let candidate = d.join(file_name);
            if candidate.is_file() {
                return Some(candidate);
            }
            dir = d.parent();
        }
        None
    }
}
