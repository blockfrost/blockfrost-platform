use serde_json::Value;
use std::error::Error;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

/// FIXME: don’t use `cardano-cli`.
/// FIXME: set CARDANO_NODE_NETWORK_ID ourselves
/// FIXME: set CARDANO_NODE_SOCKET_PATH ourselves
/// FIXME: proper errors, not `Box<dyn Erro>>`

/// Check how much lovelace is on an enterprise address associated with a
/// given `payment.skey`.
pub fn lovelace_on_payment_skey(
    cardano_cli_exe: &str,
    payment_skey: &Path,
) -> Result<u64, Box<dyn Error>> {
    let address = derive_enterprise_address_from_skey(cardano_cli_exe, payment_skey)?;
    let utxo_json = query_utxo_json(cardano_cli_exe, &address)?;
    sum_lovelace_from_utxo_json(&utxo_json)
}

fn derive_enterprise_address_from_skey(
    cardano_cli_exe: &str,
    payment_skey: &Path,
) -> Result<String, Box<dyn Error>> {
    let vkey_output = Command::new(cardano_cli_exe)
        .args(["key", "verification-key", "--signing-key-file"])
        .arg(payment_skey)
        .args(["--verification-key-file", "/dev/stdout"])
        .output()?;

    if !vkey_output.status.success() {
        return Err(format!(
            "cardano-cli key verification-key failed: {}",
            String::from_utf8_lossy(&vkey_output.stderr)
        )
        .into());
    }

    let mut child = Command::new(cardano_cli_exe)
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

fn query_utxo_json(cardano_cli_exe: &str, address: &str) -> Result<String, Box<dyn Error>> {
    let output = Command::new(cardano_cli_exe)
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
                    .checked_add(as_u64(lovelace_val)?)
                    .ok_or("cannot add".to_string())?;
                continue;
            }
        }

        if let Some(amount_arr) = utxo.get("amount").and_then(|v| v.as_array()) {
            if let Some(lovelace_val) = amount_arr.get(0) {
                total = total
                    .checked_add(as_u64(lovelace_val)?)
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
