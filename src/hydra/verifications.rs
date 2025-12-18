use serde_json::Value;
use std::path::Path;
use std::{
    error::Error,
    io::Write,
    process::{Command, Stdio},
};
use tracing::info;

/// FIXME: don’t use `cardano-cli`.
///
/// FIXME: set `CARDANO_NODE_NETWORK_ID` ourselves
///
/// FIXME: set `CARDANO_NODE_SOCKET_PATH` ourselves
///
/// FIXME: proper errors, not `Box<dyn Erro>>`
impl super::HydraManager {
    /// Generates Hydra keys if they don’t exist.
    pub(super) async fn gen_hydra_keys(&self) -> Result<(), Box<dyn Error>> {
        std::fs::create_dir_all(&self.config_dir)?;

        let key_path = self.config_dir.join("hydra.sk");

        if !key_path.exists() {
            info!("hydra-manager: generating hydra keys");

            let status = Command::new(&self.hydra_node_exe)
                .arg("gen-hydra-key")
                .arg("--output-file")
                .arg(self.config_dir.join("hydra"))
                .status()?;

            if !status.success() {
                Err(format!("gen-hydra-key failed with status: {status}"))?;
            }
        } else {
            info!("hydra-manager: hydra keys already exist");
        }

        Ok(())
    }

    /// Generates Hydra `protocol-parameters.json` if they don’t exist. These
    /// are L1 parameters with zeroed transaction fees.
    pub(super) async fn gen_protocol_parameters(&self) -> Result<(), Box<dyn Error>> {
        use serde_json::Value;

        std::fs::create_dir_all(&self.config_dir)?;

        let output = Command::new("cardano-cli")
            .args(["query", "protocol-parameters"])
            .output()?;

        if !output.status.success() {
            Err(format!("cardano-cli failed with status: {}", output.status))?;
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
        if Self::write_json_if_changed(pp_path, &params)? {
            info!("hydra-manager: protocol parameters updated");
        } else {
            info!("hydra-manager: protocol parameters unchanged");
        }

        Ok(())
    }

    /// Writes `json` to `path` (pretty-printed) **only if** the JSON content differs
    /// from what is already on disk. Returns `true` if the file was written.
    fn write_json_if_changed(
        path: impl AsRef<Path>,
        json: &serde_json::Value,
    ) -> Result<bool, Box<dyn Error>> {
        use std::fs::File;
        use std::io::Write;

        let path = path.as_ref();

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
    pub(super) fn lovelace_on_payment_skey(&self, skey_path: &Path) -> Result<u64, Box<dyn Error>> {
        let address = self.derive_enterprise_address_from_skey(skey_path)?;
        let utxo_json = self.query_utxo_json(&address)?;
        Self::sum_lovelace_from_utxo_json(&utxo_json)
    }

    fn derive_enterprise_address_from_skey(
        &self,
        skey_path: &Path,
    ) -> Result<String, Box<dyn Error>> {
        let vkey_output = Command::new(&self.cardano_cli_exe)
            .args(["key", "verification-key", "--signing-key-file"])
            .arg(skey_path)
            .args(["--verification-key-file", "/dev/stdout"])
            .output()?;

        if !vkey_output.status.success() {
            return Err(format!(
                "cardano-cli key verification-key failed: {}",
                String::from_utf8_lossy(&vkey_output.stderr)
            )
            .into());
        }

        let mut child = Command::new(&self.cardano_cli_exe)
            .args([
                "address",
                "build",
                "--payment-verification-key-file",
                "/dev/stdin",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;

        {
            let stdin = child
                .stdin
                .as_mut()
                .ok_or("failed to open stdin for cardano-cli address build")?;
            stdin.write_all(&vkey_output.stdout)?;
        }

        let addr_output = child.wait_with_output()?;
        if !addr_output.status.success() {
            return Err(format!(
                "cardano-cli address build failed: {}",
                String::from_utf8_lossy(&addr_output.stderr)
            )
            .into());
        }

        let address = String::from_utf8(addr_output.stdout)?.trim().to_string();
        if address.is_empty() {
            return Err("derived address is empty".into());
        }

        Ok(address)
    }

    fn query_utxo_json(&self, address: &str) -> Result<String, Box<dyn Error>> {
        let output = Command::new(&self.cardano_cli_exe)
            .args(["query", "utxo", "--address"])
            .arg(address)
            .args(["--output-json"])
            .output()?;

        if !output.status.success() {
            return Err(format!(
                "cardano-cli query utxo failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )
            .into());
        }

        Ok(String::from_utf8(output.stdout)?)
    }

    fn sum_lovelace_from_utxo_json(json: &str) -> Result<u64, Box<dyn Error>> {
        let v: Value = serde_json::from_str(json)?;
        let obj = v.as_object().ok_or("UTxO JSON root is not an object")?;

        let mut total: u64 = 0;

        for (_txin, utxo) in obj {
            if let Some(value_obj) = utxo.get("value").and_then(|v| v.as_object()) {
                if let Some(lovelace_val) = value_obj.get("lovelace") {
                    total = total
                        .checked_add(Self::as_u64(lovelace_val)?)
                        .ok_or("cannot add".to_string())?;
                    continue;
                }
            }

            if let Some(amount_arr) = utxo.get("amount").and_then(|v| v.as_array()) {
                if let Some(lovelace_val) = amount_arr.first() {
                    total = total
                        .checked_add(Self::as_u64(lovelace_val)?)
                        .ok_or("cannot add".to_string())?;
                }
            }
        }

        Ok(total)
    }

    /// Convert a JSON value into u64, allowing either number or string.
    fn as_u64(v: &Value) -> Result<u64, Box<dyn Error>> {
        if let Some(n) = v.as_u64() {
            return Ok(n);
        }
        if let Some(s) = v.as_str() {
            return Ok(s.parse()?);
        }
        Err("lovelace value is neither u64 nor string".into())
    }
}
