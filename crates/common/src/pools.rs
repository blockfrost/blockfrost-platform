use crate::errors::BlockfrostError;
use bech32::{ToBase32, decode};
use serde::Deserialize;

#[derive(Deserialize, Clone)]
pub struct PoolsPath {
    pub pool_id: String,
}

pub struct PoolData {
    pub pool_id: String,
}

impl PoolData {
    pub fn from_path(pool_id: &str) -> Result<Self, BlockfrostError> {
        let pool_id = match Self::validate_and_convert_pool(pool_id) {
            Some(pool_id) => Ok(pool_id),
            None => Err(BlockfrostError::invalid_pool_id()),
        }?;

        Ok(Self { pool_id })
    }

    pub fn validate_and_convert_pool(input: &str) -> Option<String> {
        if hex::decode(input).is_ok() {
            let words = match hex::decode(input) {
                Ok(bytes) => bytes.to_base32(),
                _ => return None,
            };
            let pool_id = match bech32::encode("pool", words, bech32::Variant::Bech32) {
                Ok(pool_id) => pool_id,
                Err(_) => return None,
            };

            Some(pool_id)
        } else {
            let bech32_info = decode(input).ok()?;

            if bech32_info.0 == "pool" {
                return Some(input.to_string());
            }

            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    #[rstest]
    #[case("Valid pool Bech32", "pool1pu5jlj4q9w9jlxeu370a3c9myx47md5j5m2str0naunn2q3lkdy", Some("pool1pu5jlj4q9w9jlxeu370a3c9myx47md5j5m2str0naunn2q3lkdy".to_string()))]
    #[case("Valid pool Hex", "0f292fcaa02b8b2f9b3c8f9fd8e0bb21abedb692a6d5058df3ef2735", Some("pool1pu5jlj4q9w9jlxeu370a3c9myx47md5j5m2str0naunn2q3lkdy".to_string()))]
    #[case(
        "Valid Bech32, but not pool id",
        "addr1qyw8xfunw6lhzzzsdrx5ze6j8ayxjhecv4ty5jtaey5jvwquwvnexa4lwyy9q6xdg9n4y06gd90nse2kffyhmjffycuq405jv6",
        None
    )]
    #[case("Invalid pool", "stonks_pool", None)]
    fn test_validate_and_convert_pool(
        #[case] description: &str,
        #[case] input: &str,
        #[case] expected: Option<String>,
    ) {
        assert_eq!(
            PoolData::validate_and_convert_pool(input),
            expected,
            "{}",
            description
        );
    }
}
