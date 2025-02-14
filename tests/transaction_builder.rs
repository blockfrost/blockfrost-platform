use cardano_serialization_lib::{BaseAddress, Bip32PrivateKey, Credential, NetworkId, PrivateKey};
use bip39::{ Language, Mnemonic};

#[derive(PartialEq, Eq)]
enum Network {
    Mainnet,
    Preview,
}

fn harden(number: u32) -> u32 {
    0x80_00_00_00 + number
}

fn derive_address_private_key(
    bip_prv_key: Bip32PrivateKey,
    network: Network,
    address_index: u32,
) -> (PrivateKey, String) {
    let account_index = 0;

    let network_id: u8 = if network == Network::Mainnet {
        NetworkId::mainnet().to_bytes()[0]
    } else {
        NetworkId::testnet().to_bytes()[0]
    };

    let account_key = bip_prv_key
        .derive(harden(1852))
        .derive(harden(1815))
        .derive(harden(account_index));

    let utxo_key = account_key.derive(0).derive(address_index);
    let stake_key = account_key.derive(2).derive(0).to_public();

    let utxo_pub = utxo_key.to_public();
    let utxo_raw = utxo_pub.to_raw_key();
    let stake_raw = stake_key.to_raw_key();

    let utxo_cred = Credential::from_keyhash(&utxo_raw.hash());
    let stake_cred = Credential::from_keyhash(&stake_raw.hash());

    let base_address = BaseAddress::new(network_id, &utxo_cred, &stake_cred);
    let address = base_address.to_address().to_bech32(None).unwrap();

    (utxo_pub, address)
}

fn mnemonic_to_bip32_private_key(mnemonic: &str) -> Bip32PrivateKey {
    let mnemonic = Mnemonic::from_entropy(mnemonic, Language::English)
        .expect("Invalid mnemonic phrase");
    let entropy = mnemonic.entropy();

    Bip32PrivateKey::from_bip39_entropy(entropy, &[])
}