use crate::{config::Network, errors::BlockfrostError, payment_cred::PaymentCred};

use bech32::decode;
use core::fmt;
use pallas_addresses::ByronAddress;
use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub enum AddressType {
    Invalid,
    Byron,
    Shelley,
}

#[derive(Deserialize, Debug, Clone)]
pub struct AddressesPath {
    pub address: String,
    pub asset: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct AddressInfo {
    pub address_type: AddressType,
    pub payment_cred: PaymentCred,
}

impl AddressInfo {
    pub fn from_address(address: &str, network: Network) -> Result<Self, BlockfrostError> {
        let address_type = Self::get_address_type(address, network);

        if address_type == AddressType::Invalid {
            return Err(BlockfrostError::invalid_address());
        }

        let payment_cred = PaymentCred::from_bech_32(address);

        Ok(Self {
            address_type,
            payment_cred,
        })
    }

    pub fn get_address_type(address: &str, network: Network) -> AddressType {
        if ByronAddress::from_base58(address).is_ok() {
            return AddressType::Byron;
        }

        decode(address).map_or(AddressType::Invalid, |info| {
            let (prefix, _, _) = info;

            match prefix.as_str() {
                "addr" if network == Network::Mainnet => AddressType::Shelley,
                "addr_test" if network != Network::Mainnet => AddressType::Shelley,
                "addr_vkh" | "script" => AddressType::Shelley,
                _ => AddressType::Invalid,
            }
        })
    }
}

impl fmt::Display for AddressType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Byron => write!(f, "byron"),
            Self::Shelley => write!(f, "shelley"),
            Self::Invalid => write!(f, "shelley"),
        }
    }
}

pub fn is_stake_address_valid(input: &str, network: &Network) -> Result<bool, BlockfrostError> {
    let bech32_info = decode(input).map_err(|_| BlockfrostError::invalid_stake_address())?;
    let prefix_str = match bech32_info.0.as_ref() {
        "stake" => Ok("stake"),
        "stake_test" => Ok("stake_test"),
        _ => Err(bech32::Error::InvalidLength),
    }
    .map_err(|_| BlockfrostError::invalid_stake_address())?;

    match network {
        Network::Mainnet if prefix_str == "stake" => Ok(true),
        Network::Preprod | Network::Preview if prefix_str == "stake_test" => Ok(true),
        _ => Ok(false),
    }
}

#[cfg(test)]
mod tests {
    use super::{AddressType, is_stake_address_valid};
    use crate::addresses::AddressInfo;
    use crate::config::Network;
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    #[rstest]
    #[case(
        "Valid byron address",
        Network::Mainnet,
        "DdzFFzCqrhstmqBkaU98vdHu6PdqjqotmgudToWYEeRmQKDrn4cAgGv9EZKtu1DevLrMA1pdVazufUCK4zhFkUcQZ5Gm88mVHnrwmXvT",
        AddressType::Byron
    )]
    #[case(
        "Weird invalid byron address 1",
        Network::Preprod,
        "DdzFFzCqrhstmqBkaU98vdHu6PdqjqotmgudToWYEeRm",
        AddressType::Invalid
    )]
    #[case(
        "Weird invalid byron address 2",
        Network::Preprod,
        "9wF34Fu6dz7BXN5yLYsxmpaxxvQ2S4fk8Kst4jxiu8qv",
        AddressType::Invalid
    )]
    #[case(
        "Weird invalid byron address 3",
        Network::Preprod,
        "A5TF598qPxNKKeEdrNQxxTpVNPrzsByxaerM9T89XGJg",
        AddressType::Invalid
    )]
    #[case(
        "Weird invalid byron address 4",
        Network::Preprod,
        "Au69MPHvJZdA2ahYzpZ2HCK4TdBXnNmstBFWxTMAKAAo",
        AddressType::Invalid
    )]
    #[case(
        "Valid preprod address",
        Network::Preprod,
        "addr_test1wrrgep77m0v8uv5unauluwgyr7pmdr2827wgye3sx5aw7yg7z2dsu",
        AddressType::Shelley
    )]
    #[case(
        "Valid shelley address",
        Network::Mainnet,
        "addr1qyw8xfunw6lhzzzsdrx5ze6j8ayxjhecv4ty5jtaey5jvwquwvnexa4lwyy9q6xdg9n4y06gd90nse2kffyhmjffycuq405jv6",
        AddressType::Shelley
    )]
    #[case(
        "Valid paymentCred address",
        Network::Mainnet,
        "addr_vkh1r3ej0ymkhacss5rge4qkw53lfp547wr92e9yjlwf9ynrsk5q93m",
        AddressType::Shelley
    )]
    #[case(
        "Valid paymentCred address (addr_vk)",
        Network::Mainnet,
        "addr_vk1w0l2sr2zgfm26ztc6nl9xy8ghsk5sh6ldwemlpmp9xylzy4dtf7st80zhd",
        AddressType::Invalid
    )]
    #[case(
        "Valid address, wrong network",
        Network::Preprod,
        "addr1qyw8xfunw6lhzzzsdrx5ze6j8ayxjhecv4ty5jtaey5jvwquwvnexa4lwyy9q6xdg9n4y06gd90nse2kffyhmjffycuq405jv6",
        AddressType::Invalid
    )]
    #[case(
        "Non valid/malformed address",
        Network::Mainnet,
        "stonks_address",
        AddressType::Invalid
    )]
    #[case(
        "TESTNET: Valid address",
        Network::Preprod,
        "addr_test1qryydf62jprgmtfq02370u07ch8kluvjvm4zx7gn8gmpd9snea2aza02sj9c0h4nay20a0t7q28zhajng36a2taec0gqeywmev",
        AddressType::Shelley
    )]
    #[case(
        "TESTNET: Valid paymentCred address",
        Network::Preprod,
        "addr_vkh1epr2wj5sg6x66gr650nlrlk9eahlrynxag3hjye6xctfvmdduge",
        AddressType::Shelley
    )]
    #[case(
        "TESTNET: Valid address, wrong network",
        Network::Mainnet,
        "addr_test1qryydf62jprgmtfq02370u07ch8kluvjvm4zx7gn8gmpd9snea2aza02sj9c0h4nay20a0t7q28zhajng36a2taec0gqeywmev",
        AddressType::Invalid
    )]
    #[case(
        "TESTNET: Non valid/malformed address",
        Network::Preprod,
        "stonks_address_testnet",
        AddressType::Invalid
    )]
    fn test_get_address_type(
        #[case] description: &str,
        #[case] network: Network,
        #[case] input: &str,
        #[case] expected: AddressType,
    ) {
        assert_eq!(
            AddressInfo::get_address_type(input, network),
            expected,
            "{}",
            description
        );
    }

    #[rstest]
    #[case(
        "valid stake address",
        "stake1uxmdw34s0rkc26d9x9aax69pcua8eukm2tytlx3szg75mcg5z5nss",
        Network::Mainnet,
        true
    )]
    #[case(
        "wrong network",
        "stake1uxmdw34s0rkc26d9x9aax69pcua8eukm2tytlx3szg75mcg5z5nss",
        Network::Preview,
        false
    )]
    #[case(
        "Non valid/malformed stake address",
        "stake_stonks",
        Network::Mainnet,
        false
    )]
    #[case(
        "TESTNET: valid stake address",
        "stake_test1urtemlwr6hmw6q5mc5p0q6z06g4f3v33czec67yf688w4wsw6rnpq",
        Network::Preprod,
        true
    )]
    #[case(
        "TESTNET: valid stake address, wrong network",
        "stake_test1uzxpncx82vfkl5ml00ws44hzfdh64r22kr93e79jqsumv0q8g8cy08878787",
        Network::Mainnet,
        false
    )]
    #[case(
        "TESTNET: Non valid/malformed stake address",
        "stake_stonks_testnet",
        Network::Preprod,
        false
    )]
    fn test_validate_stake_address(
        #[case] description: &str,
        #[case] input: &str,
        #[case] network: Network,
        #[case] expected: bool,
    ) {
        let result = is_stake_address_valid(input, &network);

        match result {
            Ok(value) => assert_eq!(value, expected, "{}", description),
            Err(_) => assert!(!expected, "{}", description),
        }
    }
}
