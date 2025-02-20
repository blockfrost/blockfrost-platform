use anyhow::{anyhow, Result};
use bip39::Mnemonic;
use blockfrost::{BlockfrostAPI, Pagination};
use blockfrost_openapi::models::{AddressUtxoContentInner, EpochParamContent};
use cardano_serialization_lib::{
    hash_transaction, make_vkey_witness, Address, BaseAddress, BigNum, Bip32PrivateKey,
    CoinSelectionStrategyCIP2, Credential, LinearFee, NetworkId, PrivateKey, Transaction,
    TransactionBody, TransactionBuilder, TransactionBuilderConfigBuilder, TransactionHash,
    TransactionInput, TransactionOutput, TransactionUnspentOutput, TransactionUnspentOutputs,
    TransactionWitnessSet, Vkeywitnesses,
};

pub async fn build_tx(blockfrost_client: &BlockfrostAPI) -> Result<Transaction> {
    let bip32_prv_key = mnemonic_to_private_key(
        "bright despair immune pause column saddle legal minimum erode thank silver ordinary pet next symptom second grow chapter fiber donate humble syrup glad early",
    )?;

    let (sign_key, address) = derive_address_private_key(bip32_prv_key, 0);
    let protocol_parameters = blockfrost_client.epochs_latest_parameters().await?;

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
        return Err(anyhow!(
            "You should send ADA to {} to have enough funds to send a transaction",
            address
        ));
    }

    let latest_block = blockfrost_client.blocks_latest().await?;
    let current_slot = latest_block
        .slot
        .ok_or_else(|| anyhow!("Latest block is missing a slot value"))?
        as u64;

    let (_tx_hash, tx_body) = compose_transaction(
        &address,
        &address,
        "1000000",
        &utxos,
        &protocol_parameters,
        current_slot,
    )?;

    let transaction = sign_transaction(&tx_body, &sign_key);
    Ok(transaction)
}

pub fn compose_transaction(
    address: &str,
    output_address: &str,
    output_amount: &str,
    utxos: &[AddressUtxoContentInner],
    params: &EpochParamContent,
    current_slot: u64,
) -> Result<(String, TransactionBody)> {
    if utxos.is_empty() {
        return Err(anyhow!("No UTXO on address {}", address));
    }

    let config = TransactionBuilderConfigBuilder::new()
        .fee_algo(&LinearFee::new(
            &BigNum::from_str(&params.min_fee_a.to_string())?,
            &BigNum::from_str(&params.min_fee_b.to_string())?,
        ))
        .pool_deposit(&BigNum::from_str(&params.pool_deposit)?)
        .key_deposit(&BigNum::from_str(&params.key_deposit)?)
        .coins_per_utxo_byte(&BigNum::from_str(
            params
                .coins_per_utxo_size
                .as_ref()
                .ok_or_else(|| anyhow!("coins_per_utxo_size missing"))?,
        )?)
        .max_value_size(
            params
                .max_val_size
                .as_ref()
                .ok_or_else(|| anyhow!("max_val_size missing"))?
                .parse::<u32>()?,
        )
        .max_tx_size(params.max_tx_size.try_into()?)
        .build()?;

    let mut tx_builder = TransactionBuilder::new(&config);

    let output_addr = Address::from_bech32(output_address)?;
    let change_addr = Address::from_bech32(address)?;

    let ttl = current_slot + 7200;
    tx_builder.set_ttl_bignum(&BigNum::from_str(&ttl.to_string())?);

    let output_value = cardano_serialization_lib::Value::new(&BigNum::from_str(output_amount)?);
    let tx_output = TransactionOutput::new(&output_addr, &output_value);
    tx_builder.add_output(&tx_output)?;

    let lovelace_utxos: Vec<&AddressUtxoContentInner> = utxos
        .iter()
        .filter(|u| u.amount.iter().all(|a| a.unit == "lovelace"))
        .collect();

    let mut unspent_outputs = TransactionUnspentOutputs::new();
    for utxo in lovelace_utxos {
        if let Some(token) = utxo.amount.iter().find(|a| a.unit == "lovelace") {
            let input_value =
                cardano_serialization_lib::Value::new(&BigNum::from_str(&token.quantity)?);
            let tx_hash_bytes = hex::decode(&utxo.tx_hash)?;
            let tx_hash = TransactionHash::from_bytes(tx_hash_bytes)?;
            let input = TransactionInput::new(&tx_hash, utxo.output_index.try_into()?);
            let output = TransactionOutput::new(&change_addr, &input_value);
            let unspent = TransactionUnspentOutput::new(&input, &output);
            unspent_outputs.add(&unspent);
        }
    }

    tx_builder.add_inputs_from(&unspent_outputs, CoinSelectionStrategyCIP2::LargestFirst)?;
    tx_builder.add_change_if_needed(&change_addr)?;

    let tx_body = tx_builder.build()?;
    let tx_hash = hex::encode(hash_transaction(&tx_body).to_bytes());
    Ok((tx_hash, tx_body))
}

fn harden(number: u32) -> u32 {
    0x80_00_00_00 + number
}

fn derive_address_private_key(
    bip_prv_key: Bip32PrivateKey,
    address_index: u32,
) -> (PrivateKey, String) {
    let account_index = 0;
    let network_id: u8 = NetworkId::testnet().to_bytes()[0];
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

    let sign_key = utxo_key.to_raw_key();
    (sign_key, address)
}

pub fn mnemonic_to_private_key(mnemonic_str: &str) -> Result<Bip32PrivateKey> {
    let mnemonic = Mnemonic::parse_normalized(mnemonic_str)?;
    let entropy = mnemonic.to_entropy();

    Ok(Bip32PrivateKey::from_bip39_entropy(&entropy, &[]))
}

pub fn sign_transaction(tx_body: &TransactionBody, sign_key: &PrivateKey) -> Transaction {
    let tx_hash = hash_transaction(tx_body);

    let mut witnesses = TransactionWitnessSet::new();
    let mut vkey_witnesses = Vkeywitnesses::new();

    vkey_witnesses.add(&make_vkey_witness(&tx_hash, sign_key));
    witnesses.set_vkeys(&vkey_witnesses);

    Transaction::new(tx_body, &witnesses, None)
}
