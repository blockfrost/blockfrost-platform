use anyhow::Error;
use bip39::{Language, Mnemonic};
use blockfrost::{BlockfrostAPI, Pagination};
use blockfrost_openapi::models::{AddressUtxoContentInner, EpochParamContent};
use cardano_serialization_lib::{
    make_vkey_witness, Address, BaseAddress, BigNum, Bip32PrivateKey, CoinSelectionStrategyCIP2,
    Credential, LinearFee, NetworkId, PrivateKey, Transaction, TransactionBody, TransactionBuilder,
    TransactionBuilderConfigBuilder, TransactionHash, TransactionInput, TransactionOutput,
    TransactionUnspentOutput, TransactionUnspentOutputs, TransactionWitnessSet, Vkeywitnesses,
};

/// Simple enum for network selection.
#[derive(PartialEq, Eq)]
pub enum Network {
    Mainnet,
    Preview,
}

pub async fn build_tx(
    blockfrost_client: &BlockfrostAPI,
    network: Network,
) -> Result<Transaction, Error> {
    // Derive the private key from a mnemonic.
    let bip32_prv_key = mnemonic_to_bip32_private_key(
        "bright despair immune pause column saddle legal minimum erode thank silver ordinary pet next symptom second grow chapter fiber donate humble syrup glad early",
    );

    // Use index 0 for derivation.
    let (sign_key, address) = derive_address_private_key(bip32_prv_key, network, 0);

    // Fetch the protocol parameters.
    let protocol_parameters = blockfrost_client.epochs_latest_parameters().await?;

    // Fetch UTXOs for the address.
    let utxos = blockfrost_client
        .addresses_utxos(&address, Pagination::all())
        .await?;

    let has_low_balance = utxos.len() == 1 && {
        let lovelace_amount = utxos[0]
            .amount
            .iter()
            .find(|a| a.unit == "lovelace")
            .map(|a| a.quantity.parse::<u64>().unwrap_or(0))
            .unwrap_or(0);
        lovelace_amount < 2_000_000
    };

    if utxos.is_empty() || has_low_balance {
        return Err(anyhow::anyhow!(
            "You should send ADA to {} to have enough funds to send a transaction",
            address
        ));
    }

    // Get the current slot from the latest block.
    let latest_block = blockfrost_client.blocks_latest().await?;
    let current_slot = latest_block.slot;

    // Compose the transaction using the fetched parameters.
    let (_tx_hash, tx_body) = compose_transaction(
        &address,
        &address,
        "1000000",
        &utxos,
        &protocol_parameters,
        current_slot,
    )?;

    // Sign the transaction.
    let transaction = sign_transaction(&tx_body, &sign_key);

    Ok(transaction)
}

/// Compose a transaction given source/destination addresses, UTXOs, protocol parameters, and current slot.
pub fn compose_transaction(
    address: &str,
    output_address: &str,
    output_amount: &str,
    utxos: &[AddressUtxoContentInner],
    params: &EpochParamContent,
    current_slot: u64,
) -> Result<(String, TransactionBody), Box<dyn std::error::Error>> {
    if utxos.is_empty() {
        return Err(format!("No UTXO on address {}", address).into());
    }

    // Build the transaction configuration.
    let config = TransactionBuilderConfigBuilder::new()
        .fee_algo(&LinearFee::new(
            BigNum::from_bytes(params.min_fee_a),
            params.min_fee_b,
        ))
        .pool_deposit(&BigNum::from_str(&params.protocol_params.pool_deposit)?)
        .key_deposit(&BigNum::from_str(&params.protocol_params.key_deposit)?)
        .coins_per_utxo_byte(&BigNum::from_str(
            &params.protocol_params.coins_per_utxo_size,
        )?)
        .max_value_size(params.protocol_params.max_val_size.parse()?)
        .max_tx_size(params.protocol_params.max_tx_size)
        .build();

    let mut tx_builder = TransactionBuilder::new(&config);

    // Convert bech32 addresses.
    let output_addr = Address::from_bech32(output_address)?;
    let change_addr = Address::from_bech32(address)?;

    // Set the transaction TTL (+2 hours from the current slot).
    let ttl = current_slot + 7200;
    tx_builder.set_ttl_bignum(&BigNum::from_str(&ttl.to_string())?);

    // Add the desired output.
    let output_value = cardano_serialization_lib::Value::new(&BigNum::from_str(output_amount)?);
    let tx_output = TransactionOutput::new(&output_addr, &output_value);
    tx_builder.add_output(&tx_output)?;

    // Filter UTXOs: keep only those containing only lovelace.
    let lovelace_utxos: Vec<&Utxo> = utxos
        .iter()
        .filter(|u| u.amount.iter().all(|a| a.unit == "lovelace"))
        .collect();

    let mut unspent_outputs = TransactionUnspentOutputs::new();

    // Create TransactionUnspentOutputs from the filtered UTXOs.
    for utxo in lovelace_utxos {
        if let Some(token) = utxo.amount.iter().find(|a| a.unit == "lovelace") {
            let input_value =
                cardano_serialization_lib::Value::new(&BigNum::from_str(&token.quantity)?);
            let tx_hash_bytes = hex::decode(&utxo.tx_hash)?;
            let tx_hash = TransactionHash::from_bytes(tx_hash_bytes.as_slice())?;
            let input = TransactionInput::new(&tx_hash, utxo.output_index);
            let output = TransactionOutput::new(&change_addr, &input_value);
            let unspent = TransactionUnspentOutput::new(&input, &output);
            unspent_outputs.add(&unspent);
        }
    }

    // Select inputs using a largest-first strategy.
    tx_builder.add_inputs_from(&unspent_outputs, CoinSelectionStrategyCIP2::LargestFirst)?;
    // Add a change output if needed.
    tx_builder.add_change_if_needed(&change_addr)?;

    // Build the transaction body.
    let tx_body = tx_builder.build()?;
    // Calculate the transaction hash.
    let tx_hash = hex::encode(hash_transaction(&tx_body).to_bytes());

    Ok((tx_hash, tx_body))
}

/// Helper to hash a transaction body.
fn hash_transaction(tx_body: &TransactionBody) -> TransactionHash {
    tx_body
}

/// Helper for hardened derivation.
fn harden(number: u32) -> u32 {
    0x80_00_00_00 + number
}

/// Derives a signing key and address from a bip32 private key.
fn derive_address_private_key(
    bip_prv_key: Bip32PrivateKey,
    network: Network,
    address_index: u32,
) -> (PrivateKey, String) {
    let account_index = 0;
    let network_id: u8 = match network {
        Network::Mainnet => NetworkId::mainnet().to_bytes()[0],
        Network::Preview => NetworkId::testnet().to_bytes()[0],
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

/// Converts a mnemonic phrase into a Bip32PrivateKey.
fn mnemonic_to_bip32_private_key(mnemonic: &str) -> Bip32PrivateKey {
    // Here we parse the mnemonic phrase using the English wordlist.
    let mnemonic =
        Mnemonic::from_phrase(mnemonic, Language::English).expect("Invalid mnemonic phrase");
    let entropy = mnemonic.entropy();
    Bip32PrivateKey::from_bip39_entropy(entropy, &[])
}

/// Signs a transaction body with the provided signing key.
pub fn sign_transaction(tx_body: &TransactionBody, sign_key: &PrivateKey) -> Transaction {
    let tx_hash = hash_transaction(tx_body);
    let mut witnesses = TransactionWitnessSet::new();
    let mut vkey_witnesses = Vkeywitnesses::new();

    vkey_witnesses.add(&make_vkey_witness(&tx_hash, sign_key));
    witnesses.set_vkeys(&vkey_witnesses);

    Transaction::new(tx_body, &witnesses)
}
