use anyhow::{Result, anyhow};
use serde_json::Value;
use std::path::Path;
use tracing::info;

/// FIXME: don’t use `cardano-cli`.
///
/// FIXME: proper errors, not `anyhow!`
impl super::State {
    /// Generates Hydra keys if they don’t exist.
    pub(super) async fn gen_hydra_keys(&self) -> Result<()> {
        std::fs::create_dir_all(&self.config_dir)?;

        let key_path = self.config_dir.join("hydra.sk");

        if !key_path.exists() {
            info!("hydra-controller: generating hydra keys");

            let status = tokio::process::Command::new(&self.hydra_node_exe)
                .arg("gen-hydra-key")
                .arg("--output-file")
                .arg(self.config_dir.join("hydra"))
                .status()
                .await?;

            if !status.success() {
                Err(anyhow!("gen-hydra-key failed with status: {status}"))?;
            }
        } else {
            info!("hydra-controller: hydra keys already exist");
        }

        Ok(())
    }

    fn cardano_cli_env(&self) -> Vec<(&str, String)> {
        vec![
            ("CARDANO_NODE_SOCKET_PATH", self.node_socket_path.clone()),
            (
                "CARDANO_NODE_NETWORK_ID",
                match &self.network {
                    crate::types::Network::Mainnet => self.network.as_str().to_string(),
                    _ => self.network.network_magic().to_string(),
                },
            ),
        ]
    }

    /// Check how much lovelace is on an enterprise address associated with a
    /// given `payment.skey`.
    pub(super) async fn lovelace_on_payment_skey(&self, skey_path: &Path) -> Result<u64> {
        let address = self.derive_enterprise_address_from_skey(skey_path).await?;
        let utxo_json = self.query_utxo_json(&address).await?;
        Self::sum_lovelace_from_utxo_json(&utxo_json)
    }

    pub(super) async fn derive_vkey_from_skey(
        &self,
        skey_path: &Path,
    ) -> Result<serde_json::Value> {
        let vkey_output = tokio::process::Command::new(&self.cardano_cli_exe)
            .envs(self.cardano_cli_env())
            .args(["key", "verification-key", "--signing-key-file"])
            .arg(skey_path)
            .args(["--verification-key-file", "/dev/stdout"])
            .output()
            .await?;
        Ok(serde_json::from_slice(&vkey_output.stdout)?)
    }

    async fn derive_enterprise_address_from_skey(&self, skey_path: &Path) -> Result<String> {
        let vkey_output = tokio::process::Command::new(&self.cardano_cli_exe)
            .envs(self.cardano_cli_env())
            .args(["key", "verification-key", "--signing-key-file"])
            .arg(skey_path)
            .args(["--verification-key-file", "/dev/stdout"])
            .output()
            .await?;

        if !vkey_output.status.success() {
            return Err(anyhow!(
                "cardano-cli key verification-key failed: {}",
                String::from_utf8_lossy(&vkey_output.stderr)
            ));
        }

        let mut child = tokio::process::Command::new(&self.cardano_cli_exe)
            .envs(self.cardano_cli_env())
            .args([
                "address",
                "build",
                "--payment-verification-key-file",
                "/dev/stdin",
            ])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()?;

        {
            let stdin = child.stdin.as_mut().ok_or(anyhow!(
                "failed to open stdin for cardano-cli address build"
            ))?;
            use tokio::io::AsyncWriteExt;
            stdin.write_all(&vkey_output.stdout).await?;
        }

        let addr_output = child.wait_with_output().await?;
        if !addr_output.status.success() {
            Err(anyhow!(
                "cardano-cli address build failed: {}",
                String::from_utf8_lossy(&addr_output.stderr)
            ))?;
        }

        let address = String::from_utf8(addr_output.stdout)?.trim().to_string();
        if address.is_empty() {
            return Err(anyhow!("derived address is empty"));
        }

        Ok(address)
    }

    async fn query_utxo_json(&self, address: &str) -> Result<String> {
        let output = tokio::process::Command::new(&self.cardano_cli_exe)
            .envs(self.cardano_cli_env())
            .args(["query", "utxo", "--address"])
            .arg(address)
            .args(["--output-json"])
            .output()
            .await?;

        if !output.status.success() {
            return Err(anyhow!(
                "cardano-cli query utxo failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        Ok(String::from_utf8(output.stdout)?)
    }

    fn sum_lovelace_from_utxo_json(json: &str) -> Result<u64> {
        let v: Value = serde_json::from_str(json)?;
        let obj = v
            .as_object()
            .ok_or(anyhow!("UTxO JSON root is not an object"))?;

        let mut total: u64 = 0;

        for (_txin, utxo) in obj {
            if let Some(value_obj) = utxo.get("value").and_then(|v| v.as_object()) {
                if let Some(lovelace_val) = value_obj.get("lovelace") {
                    total = total
                        .checked_add(Self::as_u64(lovelace_val)?)
                        .ok_or(anyhow!("cannot add"))?;
                    continue;
                }
            }

            if let Some(amount_arr) = utxo.get("amount").and_then(|v| v.as_array()) {
                if let Some(lovelace_val) = amount_arr.first() {
                    total = total
                        .checked_add(Self::as_u64(lovelace_val)?)
                        .ok_or(anyhow!("cannot add"))?;
                }
            }
        }

        Ok(total)
    }

    /// Convert a JSON value into u64, allowing either number or string.
    fn as_u64(v: &Value) -> Result<u64> {
        if let Some(n) = v.as_u64() {
            return Ok(n);
        }
        if let Some(s) = v.as_str() {
            return Ok(s.parse()?);
        }
        Err(anyhow!("lovelace value is neither u64 nor string"))
    }

    async fn cardano_cli_capture(
        &self,
        args: &[&str],
        stdin_bytes: Option<&[u8]>,
    ) -> Result<(serde_json::Value, Vec<u8>)> {
        use tokio::io::AsyncWriteExt;

        let mut cmd = tokio::process::Command::new(&self.cardano_cli_exe);
        cmd.envs(self.cardano_cli_env());
        cmd.args(args)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        if stdin_bytes.is_some() {
            cmd.stdin(std::process::Stdio::piped());
        } else {
            cmd.stdin(std::process::Stdio::null());
        }

        let mut child = cmd.spawn()?;

        if let Some(bytes) = stdin_bytes {
            let mut stdin = child
                .stdin
                .take()
                .ok_or_else(|| anyhow!("failed to open stdin pipe"))?;
            stdin.write_all(bytes).await?;
            stdin.shutdown().await?;
        }

        let out = child.wait_with_output().await?;

        if !out.status.success() {
            return Err(anyhow!(
                "cardano-cli failed (exit={}):\nstdout: {}\nstderr: {}",
                out.status,
                String::from_utf8_lossy(&out.stdout).trim(),
                String::from_utf8_lossy(&out.stderr).trim(),
            ));
        }

        let (json, rest) = parse_first_json_and_rest(&out.stdout)?;
        Ok((json, rest))
    }

    pub(super) async fn empty_commit_to_hydra(
        &self,
        hydra_api_port: u16,
        signing_skey: &Path,
    ) -> Result<()> {
        use anyhow::Context;
        use reqwest::header;

        let url = format!("http://127.0.0.1:{hydra_api_port}/commit");
        let client = reqwest::Client::new();
        let resp = client
            .post(url)
            .header(header::CONTENT_TYPE, "application/json")
            .body("{}")
            .send()
            .await
            .context("failed to POST /commit to hydra-node")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.bytes().await.unwrap_or_default();
            return Err(anyhow!(
                "hydra /commit failed with {}: {}",
                status,
                String::from_utf8_lossy(&body)
            ));
        }

        let commit_tx_bytes = resp
            .bytes()
            .await
            .context("failed to read hydra /commit response body")?
            .to_vec();

        let _: serde_json::Value = serde_json::from_slice(&commit_tx_bytes)
            .context("hydra /commit response was not valid JSON")?;

        let signed_tx = self
            .cardano_cli_capture(
                &[
                    "latest",
                    "transaction",
                    "sign",
                    "--tx-file",
                    "/dev/stdin",
                    "--signing-key-file",
                    signing_skey
                        .to_str()
                        .ok_or_else(|| anyhow!("commit_funds_skey is not valid UTF-8"))?,
                    "--out-file",
                    "/dev/stdout",
                ],
                Some(&commit_tx_bytes),
            )
            .await?
            .0;

        let _ = self
            .cardano_cli_capture(
                &["latest", "transaction", "submit", "--tx-file", "/dev/stdin"],
                Some(&serde_json::to_vec(&signed_tx)?),
            )
            .await?;
        Ok(())
    }
}

/// Reads a JSON file from disk.
pub fn read_json_file(path: &Path) -> Result<serde_json::Value> {
    let contents = std::fs::read_to_string(path)?;
    let json: serde_json::Value = serde_json::from_str(&contents)?;
    Ok(json)
}

/// Writes `json` to `path` (pretty-printed) **only if** the JSON content differs
/// from what is already on disk. Returns `true` if the file was written.
pub fn write_json_if_changed(path: &Path, json: &serde_json::Value) -> Result<bool> {
    use std::fs::File;
    use std::io::Write;

    if path.exists() {
        if let Ok(existing_str) = std::fs::read_to_string(path) {
            if let Ok(existing_json) = serde_json::from_str::<serde_json::Value>(&existing_str) {
                if existing_json == *json {
                    return Ok(false);
                }
            }
        }
    }

    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }

    let mut file = File::create(path)?;
    serde_json::to_writer_pretty(&mut file, json)?;
    file.write_all(b"\n")?;

    Ok(true)
}

/// Finds a free port by bind to port 0, to let the OS pick a free port.
pub async fn find_free_tcp_port() -> std::io::Result<u16> {
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0)).await?;
    let port = listener.local_addr()?.port();
    drop(listener);
    Ok(port)
}

/// Returns `Ok(true)` if `port` can be bound on 127.0.0.1 (so it's free),
/// `Ok(false)` if it's already in use, and `Err(_)` for other IO errors.
pub async fn is_tcp_port_free(port: u16) -> std::io::Result<bool> {
    match tokio::net::TcpListener::bind(("127.0.0.1", port)).await {
        Ok(listener) => {
            drop(listener);
            Ok(true)
        },
        Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => Ok(false),
        Err(e) => Err(e),
    }
}

pub async fn fetch_head_tag(hydra_api_port: u16) -> Result<String> {
    let url = format!("http://127.0.0.1:{hydra_api_port}/head");

    let v: serde_json::Value = reqwest::get(url).await?.error_for_status()?.json().await?;

    v.get("tag")
        .ok_or(anyhow!("missing tag"))
        .and_then(|a| a.as_str().ok_or(anyhow!("tag is not a string")))
        .map(|a| a.to_string())
}

/// Parse the first JSON value from e.g. stdout, and return the remainder.
fn parse_first_json_and_rest(stdout: &[u8]) -> Result<(serde_json::Value, Vec<u8>)> {
    let mut start = stdout
        .iter()
        .position(|b| !b.is_ascii_whitespace())
        .unwrap_or(0);

    if !matches!(stdout.get(start), Some(b'{') | Some(b'[')) {
        if let Some(i) = stdout.iter().position(|&b| b == b'{' || b == b'[') {
            start = i;
        }
    }

    let mut it = serde_json::Deserializer::from_slice(&stdout[start..]).into_iter::<Value>();

    let first = it
        .next()
        .ok_or_else(|| anyhow!("no JSON value found in stdout"))?
        .map_err(|e| anyhow!("failed to parse first JSON value from stdout: {e}"))?;

    let consumed = it.byte_offset(); // <-- works here
    let rest = stdout[start + consumed..].to_vec();

    Ok((first, rest))
}

#[cfg(unix)]
pub fn sigterm(pid: u32) -> Result<()> {
    use nix::sys::signal::{Signal, kill};
    use nix::unistd::Pid;
    Ok(kill(Pid::from_raw(pid as i32), Signal::SIGTERM)?)
}

#[cfg(windows)]
pub fn sigterm(_pid: u32) -> Result<()> {
    unreachable!()
}

/// We use it for `localhost` tests, to detect if the Gateway and Platform are
/// running on the same host. Then we cannot set up a
/// `[crate::hydra::tunnel2::Tunnel]`, because the ports are already taken.
pub fn hashed_machine_id() -> String {
    const MACHINE_ID_NAMESPACE: &str = "blockfrost.machine-id.v1";

    let mut hasher = blake3::Hasher::new();
    hasher.update(MACHINE_ID_NAMESPACE.as_bytes());
    hasher.update(b":");

    match machine_uid::get() {
        Ok(id) => {
            hasher.update(id.as_bytes());
        },
        Err(e) => {
            tracing::warn!(error = ?e, "machine_uid::get() failed; falling back to random bytes");
            let mut fallback = [0u8; 32];
            getrandom::fill(&mut fallback)
                .expect("getrandom::fill shouldn’t fail in normal circumstances");
            hasher.update(&fallback);
        },
    }

    hasher.finalize().to_hex().to_string()
}
