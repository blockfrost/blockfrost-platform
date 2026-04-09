//! Pure-Rust Cardano key, address, and transaction helpers.
//!
//! These replace `cardano-cli` subprocess calls for:
//! - `key verification-key` (skey→vkey)
//! - `address build` (enterprise address)
//! - `address key-gen` (new keypair)
//! - `transaction sign`
//! - Reading/writing cardano-cli JSON envelope format

use anyhow::{Result, anyhow, bail};
use cardano_serialization_lib::{
    Address, Credential, EnterpriseAddress, FixedTransaction, NetworkId, PrivateKey, PublicKey,
};
use std::path::Path;

/// The CBOR prefix for a 32-byte bytestring (`5820` in hex).
const CBOR_32_PREFIX: &str = "5820";

/// Read a cardano-cli signing-key JSON envelope and return the raw 32-byte
/// ed25519 private key.
pub fn read_skey_bytes(skey_path: &Path) -> Result<[u8; 32]> {
    let contents = std::fs::read_to_string(skey_path)?;
    let envelope: serde_json::Value = serde_json::from_str(&contents)?;
    parse_skey_envelope(&envelope)
}

/// Parse a signing-key JSON envelope value and return the raw 32-byte ed25519
/// private key.
pub fn parse_skey_envelope(envelope: &serde_json::Value) -> Result<[u8; 32]> {
    let cbor_hex = envelope
        .get("cborHex")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("skey envelope missing cborHex field"))?;

    let raw_hex = cbor_hex
        .strip_prefix(CBOR_32_PREFIX)
        .ok_or_else(|| anyhow!("skey cborHex does not start with {CBOR_32_PREFIX}"))?;

    let bytes = hex::decode(raw_hex)?;
    let arr: [u8; 32] = bytes
        .try_into()
        .map_err(|_| anyhow!("skey raw bytes are not 32 bytes"))?;
    Ok(arr)
}

/// Load a `PrivateKey` (CSL) from a cardano-cli signing-key JSON envelope file.
pub fn load_private_key(skey_path: &Path) -> Result<PrivateKey> {
    let bytes = read_skey_bytes(skey_path)?;
    PrivateKey::from_normal_bytes(&bytes).map_err(|e| anyhow!("invalid ed25519 key: {e}"))
}

/// Derive the CSL `PublicKey` from a signing-key file.
pub fn derive_public_key(skey_path: &Path) -> Result<PublicKey> {
    Ok(load_private_key(skey_path)?.to_public())
}

/// Build the cardano-cli JSON envelope for a verification key, given the raw
/// public key bytes.
pub fn vkey_envelope(pub_key: &PublicKey) -> serde_json::Value {
    let raw_bytes = pub_key.as_bytes();
    let cbor_hex = format!("{CBOR_32_PREFIX}{}", hex::encode(raw_bytes));
    serde_json::json!({
        "type": "PaymentVerificationKeyShelley_ed25519",
        "description": "Payment Verification Key",
        "cborHex": cbor_hex,
    })
}

/// Build the cardano-cli JSON envelope for a signing key, given the raw
/// private key bytes.
pub fn skey_envelope(priv_key: &PrivateKey) -> serde_json::Value {
    let raw_bytes = priv_key.as_bytes();
    let cbor_hex = format!("{CBOR_32_PREFIX}{}", hex::encode(raw_bytes));
    serde_json::json!({
        "type": "PaymentSigningKeyShelley_ed25519",
        "description": "Payment Signing Key",
        "cborHex": cbor_hex,
    })
}

/// Derive a verification-key JSON envelope from a signing-key file path.
///
/// Replaces: `cardano-cli key verification-key --signing-key-file <skey>`
pub fn derive_vkey_from_skey(skey_path: &Path) -> Result<serde_json::Value> {
    let pub_key = derive_public_key(skey_path)?;
    Ok(vkey_envelope(&pub_key))
}

/// Return the `NetworkId` (CSL) for a given network name.
pub fn network_id_for(network: &str) -> NetworkId {
    if network == "mainnet" {
        NetworkId::mainnet()
    } else {
        NetworkId::testnet()
    }
}

/// Derive the enterprise (payment-only) bech32 address from a signing-key file.
///
/// Replaces:
/// ```text
/// cardano-cli key verification-key --signing-key-file <skey> --verification-key-file /dev/stdout
/// cardano-cli address build --payment-verification-key-file /dev/stdin
/// ```
pub fn derive_enterprise_address(skey_path: &Path, network: &str) -> Result<String> {
    let pub_key = derive_public_key(skey_path)?;
    enterprise_address_from_pubkey(&pub_key, network)
}

/// Build an enterprise address from a `PublicKey`.
pub fn enterprise_address_from_pubkey(pub_key: &PublicKey, network: &str) -> Result<String> {
    let net_id = network_id_for(network);
    let key_hash = pub_key.hash();
    let cred = Credential::from_keyhash(&key_hash);
    let enterprise = EnterpriseAddress::new(net_id.to_bytes()[0], &cred);
    let addr: Address = enterprise.to_address();
    addr.to_bech32(None)
        .map_err(|e| anyhow!("failed to bech32-encode address: {e}"))
}

/// Generate a new ed25519 keypair and write them in cardano-cli JSON envelope
/// format to `<base_path>.sk` and `<base_path>.vk`.
///
/// Replaces: `cardano-cli address key-gen --signing-key-file <..>.sk --verification-key-file <..>.vk`
pub fn generate_keypair(base_path: &Path) -> Result<()> {
    let mut random_bytes = [0u8; 32];
    getrandom::fill(&mut random_bytes).map_err(|e| anyhow!("getrandom failed: {e}"))?;

    let priv_key = PrivateKey::from_normal_bytes(&random_bytes)
        .map_err(|e| anyhow!("invalid generated key: {e}"))?;
    let pub_key = priv_key.to_public();

    let sk_envelope = skey_envelope(&priv_key);
    let vk_envelope = vkey_envelope(&pub_key);

    let sk_path = base_path.with_extension("sk");
    let vk_path = base_path.with_extension("vk");

    write_json_file(&sk_path, &sk_envelope)?;
    write_json_file(&vk_path, &vk_envelope)?;

    Ok(())
}

/// Sign a transaction (in cardano-cli JSON envelope format, i.e. the bytes
/// returned by `hydra /commit` or `cardano-cli transaction build`) with a
/// signing key.
///
/// Returns the signed transaction as a cardano-cli JSON envelope value.
///
/// Replaces:
/// ```text
/// cardano-cli transaction sign --tx-file /dev/stdin --signing-key-file <skey> --out-file /dev/stdout
/// ```
pub fn sign_tx_envelope(
    tx_envelope: &serde_json::Value,
    skey_path: &Path,
) -> Result<serde_json::Value> {
    let priv_key = load_private_key(skey_path)?;
    sign_tx_envelope_with_key(tx_envelope, &priv_key)
}

/// Sign a transaction envelope with a `PrivateKey`.
pub fn sign_tx_envelope_with_key(
    tx_envelope: &serde_json::Value,
    priv_key: &PrivateKey,
) -> Result<serde_json::Value> {
    let cbor_hex = tx_envelope
        .get("cborHex")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("tx envelope missing cborHex field"))?;

    let tx_bytes = hex::decode(cbor_hex)?;
    let mut fixed_tx = FixedTransaction::from_bytes(tx_bytes)
        .map_err(|e| anyhow!("failed to parse transaction CBOR: {e}"))?;

    fixed_tx
        .sign_and_add_vkey_signature(priv_key)
        .map_err(|e| anyhow!("failed to sign transaction: {e}"))?;

    let signed_bytes = fixed_tx.to_bytes();
    let signed_hex = hex::encode(&signed_bytes);

    // Produce a "Witnessed Tx ConwayEra" envelope
    Ok(serde_json::json!({
        "type": "Witnessed Tx ConwayEra",
        "description": "",
        "cborHex": signed_hex,
    }))
}

/// Write a JSON value to a file (pretty-printed with trailing newline).
fn write_json_file(path: &Path, json: &serde_json::Value) -> Result<()> {
    use std::fs::File;
    use std::io::Write;

    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)?;
    }

    let mut file = File::create(path)?;
    serde_json::to_writer_pretty(&mut file, json)?;
    file.write_all(b"\n")?;
    Ok(())
}

/// Sum lovelace from a Blockfrost-style address UTxO response.
///
/// Each item has `.amount[]` with `{unit: "lovelace", quantity: "..."}`.
pub fn sum_lovelace_from_blockfrost_utxos(
    utxos: &[bf_api_provider::types::TestsAddressUtxoResponse],
) -> u64 {
    utxos
        .iter()
        .flat_map(|u| u.amount.iter())
        .filter(|a| a.unit == "lovelace")
        .filter_map(|a| a.quantity.parse::<u64>().ok())
        .sum()
}

/// Convert a cardano-cli `query utxo --output-json` style JSON object into a
/// lovelace total.
///
/// Handles both the `{value: {lovelace: N}}` shape and the
/// `{amount: [{unit, quantity}]}` shape.
pub fn sum_lovelace_from_utxo_json(json: &serde_json::Value) -> Result<u64> {
    let obj = json
        .as_object()
        .ok_or_else(|| anyhow!("UTxO JSON root is not an object"))?;

    let mut total: u64 = 0;

    for (_txin, utxo) in obj {
        if let Some(value_obj) = utxo.get("value").and_then(|v| v.as_object())
            && let Some(lovelace_val) = value_obj.get("lovelace")
        {
            total = total
                .checked_add(json_as_u64(lovelace_val)?)
                .ok_or_else(|| anyhow!("lovelace sum overflow"))?;
            continue;
        }

        if let Some(amount_arr) = utxo.get("amount").and_then(|v| v.as_array())
            && let Some(lovelace_val) = amount_arr.first()
        {
            total = total
                .checked_add(json_as_u64(lovelace_val)?)
                .ok_or_else(|| anyhow!("lovelace sum overflow"))?;
        }
    }

    Ok(total)
}

/// Convert a JSON value into u64, allowing either number or string.
pub fn json_as_u64(v: &serde_json::Value) -> Result<u64> {
    if let Some(n) = v.as_u64() {
        return Ok(n);
    }
    if let Some(s) = v.as_str() {
        return Ok(s.parse()?);
    }
    bail!("value is neither u64 nor string")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_skey_envelope() {
        let mut random_bytes = [0u8; 32];
        getrandom::fill(&mut random_bytes).unwrap();
        let priv_key = PrivateKey::from_normal_bytes(&random_bytes).unwrap();
        let envelope = skey_envelope(&priv_key);
        let parsed = parse_skey_envelope(&envelope).unwrap();
        assert_eq!(parsed, random_bytes);
    }

    #[test]
    fn roundtrip_vkey_envelope() {
        let mut random_bytes = [0u8; 32];
        getrandom::fill(&mut random_bytes).unwrap();
        let priv_key = PrivateKey::from_normal_bytes(&random_bytes).unwrap();
        let pub_key = priv_key.to_public();
        let envelope = vkey_envelope(&pub_key);

        assert_eq!(envelope["type"], "PaymentVerificationKeyShelley_ed25519");
        let cbor_hex = envelope["cborHex"].as_str().unwrap();
        assert!(cbor_hex.starts_with(CBOR_32_PREFIX));
        let decoded = hex::decode(cbor_hex.strip_prefix(CBOR_32_PREFIX).unwrap()).unwrap();
        assert_eq!(decoded, pub_key.as_bytes());
    }

    #[test]
    fn enterprise_address_testnet() {
        let mut random_bytes = [0u8; 32];
        getrandom::fill(&mut random_bytes).unwrap();
        let priv_key = PrivateKey::from_normal_bytes(&random_bytes).unwrap();
        let pub_key = priv_key.to_public();
        let addr = enterprise_address_from_pubkey(&pub_key, "preview").unwrap();
        assert!(
            addr.starts_with("addr_test1"),
            "testnet enterprise address should start with addr_test1, got: {addr}"
        );
    }

    #[test]
    fn enterprise_address_mainnet() {
        let mut random_bytes = [0u8; 32];
        getrandom::fill(&mut random_bytes).unwrap();
        let priv_key = PrivateKey::from_normal_bytes(&random_bytes).unwrap();
        let pub_key = priv_key.to_public();
        let addr = enterprise_address_from_pubkey(&pub_key, "mainnet").unwrap();
        assert!(
            addr.starts_with("addr1"),
            "mainnet enterprise address should start with addr1, got: {addr}"
        );
    }
}
