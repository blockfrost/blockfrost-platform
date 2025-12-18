use anyhow::{Result, anyhow};
use serde_json::Value;
use std::path::Path;
use tracing::info;

/// FIXME: don’t use `cardano-cli`.
///
/// FIXME: set `CARDANO_NODE_NETWORK_ID` ourselves
///
/// FIXME: set `CARDANO_NODE_SOCKET_PATH` ourselves
///
/// FIXME: proper errors, not `Box<dyn Erro>>`
impl super::State {
    /// Generates Hydra keys if they don’t exist.
    pub(super) async fn gen_hydra_keys(&self) -> Result<()> {
        std::fs::create_dir_all(&self.config_dir)?;

        let key_path = self.config_dir.join("hydra.sk");

        if !key_path.exists() {
            info!("hydra-manager: generating hydra keys");

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
            info!("hydra-manager: hydra keys already exist");
        }

        Ok(())
    }

    /// Generates Hydra `protocol-parameters.json` if they don’t exist. These
    /// are L1 parameters with zeroed transaction fees.
    ///
    /// FIXME: move to `blockfrost-gateway`, as it controls protocol parameters for both
    pub(super) async fn _gen_protocol_parameters(&self) -> Result<()> {
        use serde_json::Value;

        std::fs::create_dir_all(&self.config_dir)?;

        let output = tokio::process::Command::new("cardano-cli")
            .args(["query", "protocol-parameters"])
            .output()
            .await?;

        if !output.status.success() {
            Err(anyhow!("cardano-cli failed with status: {}", output.status))?;
        }

        let mut params: Value = serde_json::from_slice(&output.stdout)?;

        // .txFeeFixed := 0
        // .txFeePerByte := 0
        if let Some(obj) = params.as_object_mut() {
            obj.insert("txFeeFixed".to_string(), 0.into());
            obj.insert("txFeePerByte".to_string(), 0.into());

            // .executionUnitPrices.priceMemory := 0
            // .executionUnitPrices.priceSteps := 0
            if let Some(exec_prices) = obj
                .get_mut("executionUnitPrices")
                .and_then(Value::as_object_mut)
            {
                exec_prices.insert("priceMemory".to_string(), 0.into());
                exec_prices.insert("priceSteps".to_string(), 0.into());
            }
        }

        let pp_path = self.config_dir.join("protocol-parameters.json");
        if Self::write_json_if_changed(&pp_path, &params)? {
            info!("hydra-manager: protocol parameters updated");
        } else {
            info!("hydra-manager: protocol parameters unchanged");
        }

        Ok(())
    }

    /// Reads a JSON file from disk.
    pub(super) fn read_json_file(path: &Path) -> Result<serde_json::Value> {
        let contents = std::fs::read_to_string(path)?;
        let json: serde_json::Value = serde_json::from_str(&contents)?;
        Ok(json)
    }

    /// Writes `json` to `path` (pretty-printed) **only if** the JSON content differs
    /// from what is already on disk. Returns `true` if the file was written.
    pub(super) fn write_json_if_changed(path: &Path, json: &serde_json::Value) -> Result<bool> {
        use std::fs::File;
        use std::io::Write;

        if path.exists() {
            if let Ok(existing_str) = std::fs::read_to_string(path) {
                if let Ok(existing_json) = serde_json::from_str::<serde_json::Value>(&existing_str)
                {
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
            .args(["key", "verification-key", "--signing-key-file"])
            .arg(skey_path)
            .args(["--verification-key-file", "/dev/stdout"])
            .output()
            .await?;
        Ok(serde_json::from_slice(&vkey_output.stdout)?)
    }

    async fn derive_enterprise_address_from_skey(&self, skey_path: &Path) -> Result<String> {
        let vkey_output = tokio::process::Command::new(&self.cardano_cli_exe)
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
}
