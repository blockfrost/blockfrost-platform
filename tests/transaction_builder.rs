use bip39::Mnemonic;
use blockfrost::BlockfrostAPI;
use cardano_serialization_lib::PrivateKey;
use cardano_serialization_lib::{
    Address, BigNum, CoinSelectionStrategyCIP2, LinearFee, Transaction, TransactionBody,
    TransactionBuilder, TransactionBuilderConfigBuilder, TransactionHash, TransactionInput,
    TransactionOutput, TransactionUnspentOutput, TransactionUnspentOutputs, TransactionWitnessSet,
    Value, Vkeywitnesses,
};
use cardano_serialization_lib::{BaseAddress, Bip32PrivateKey, Credential, NetworkId};
use hex;
use std::str::FromStr;

#[derive(PartialEq, Eq)]
pub enum Network {
    Mainnet,
    Preview,
}

pub fn sign_transaction(tx_body: &TransactionBody, sign_key: &PrivateKey) -> Transaction {
    let tx_hash = hash_transaction(tx_body);
    let mut witnesses = TransactionWitnessSet::new();
    let mut vkey_witnesses = Vkeywitnesses::new();

    vkey_witnesses.add(&make_vkey_witness(&tx_hash, sign_key));
    witnesses.set_vkeys(&vkey_witnesses);

    Transaction::new(tx_body, &witnesses)
}

#[derive(Debug)]
pub struct TokenAmount {
    pub unit: String,
    pub quantity: String,
}

#[derive(Debug)]
pub struct Utxo {
    pub tx_hash: String,
    pub output_index: u32,
    pub amount: Vec<TokenAmount>,
}

#[derive(Debug)]
pub struct ProtocolParams {
    pub min_fee_a: String,
    pub min_fee_b: String,
    pub pool_deposit: String,
    pub key_deposit: String,
    pub coins_per_utxo_size: String,
    pub max_val_size: String,
    pub max_tx_size: u32,
}

#[derive(Debug)]
pub struct TransactionParams {
    pub protocol_params: ProtocolParams,
    pub current_slot: u64,
}

pub async fn build_tx(
    blockfrost_client: &BlockfrostAPI,
    network: Network,
) -> Result<Transaction, Box<dyn Error>> {
    let bip32_prv_key = mnemonic_to_private_key(MNEMONIC)?;
    let env_value = env::var("TEST_ENV").unwrap_or_else(|_| "dev".to_string());

    let environment = if env_value == "prod" {
        Environment::Prod
    } else {
        Environment::Dev
    };

    let index = match environment {
        Environment::Prod => 0,
        Environment::Dev => 1,
    };

    let (sign_key, address) = derive_address_private_key(bip32_prv_key, network_str, index)?;

    let protocol_parameters: ProtocolParams = blockfrost_client.epochs_latest_parameters().await?;
    let utxos: Vec<Utxo> = blockfrost_client.addresses_utxos_all(&address).await?;

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
        return Err(format!(
            "You should send ADA to {} to have enough funds to send a transaction",
            address
        )
        .into());
    }

    let latest_block = blockfrost_client.blocks_latest().await?;
    let current_slot = latest_block.slot.ok_or("Failed to fetch slot number")?;

    // Compose the transaction.
    // Assume `compose_transaction` returns a tuple with (tx_hash, tx_body).
    let (_tx_hash, tx_body) = compose_transaction(
        &address,
        &address,
        OUTPUT_AMOUNT,
        &utxos,
        &protocol_parameters,
        current_slot,
    )?;

    // Sign the transaction.
    let transaction = sign_transaction(tx_body, &sign_key);

    Ok(transaction)
}

pub fn compose_transaction(
    address: &str,
    output_address: &str,
    output_amount: &str,
    utxos: &[Utxo],
    params: &TransactionParams,
) -> Result<(String, cardano_serialization_lib::TransactionBody), Box<dyn std::error::Error>> {
    if utxos.is_empty() {
        return Err(format!("No UTXO on address {}", address).into());
    }

    // Build transaction configuration
    let config = TransactionBuilderConfigBuilder::new()
        .fee_algo(&LinearFee::new(
            &BigNum::from_str(&params.protocol_params.min_fee_a)?,
            &BigNum::from_str(&params.protocol_params.min_fee_b)?,
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

    // Convert addresses from bech32.
    let output_addr = Address::from_bech32(output_address)?;
    let change_addr = Address::from_bech32(address)?;

    // Set TTL (+2 hours from the current slot)
    let ttl = params.current_slot + 7200;
    tx_builder.set_ttl(ttl);

    // Add output to the transaction
    let output_value = Value::new(&BigNum::from_str(output_amount)?);
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
            let input_value = Value::new(&BigNum::from_str(&token.quantity)?);
            let tx_hash_bytes = hex::decode(&utxo.tx_hash)?;
            let tx_hash = TransactionHash::from_bytes(tx_hash_bytes.as_slice())?;
            let input = TransactionInput::new(&tx_hash, utxo.output_index);
            let output = TransactionOutput::new(&change_addr, &input_value);
            let unspent = TransactionUnspentOutput::new(&input, &output);
            unspent_outputs.add(&unspent);
        }
    }

    // Add inputs using a largest-first coin selection strategy.
    tx_builder.add_inputs_from(&unspent_outputs, CoinSelectionStrategyCIP2::LargestFirst)?;

    // Add change output if needed.
    tx_builder.add_change_if_needed(&change_addr)?;

    // Build the transaction body.
    let tx_body = tx_builder.build()?;

    // Calculate transaction hash.
    let tx_hash = hex::encode(hash_transaction(&tx_body).to_bytes());

    Ok((tx_hash, tx_body))
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
    let mnemonic = Mnemonic::from_entropy(mnemonic).expect("Invalid mnemonic phrase");
    let entropy = mnemonic.entropy();

    Bip32PrivateKey::from_bip39_entropy(entropy, &[])
}
