use crate::errors::BlockfrostError;
use serde::Deserialize;

const MAX_SIGNED_INT: i64 = 2_147_483_647;

#[derive(Deserialize)]
pub struct BlocksPath {
    pub hash_or_number: String,
}

pub struct BlocksSlotPath {
    pub slot: String,
}

#[derive(Debug, PartialEq)]
pub struct BlockData {
    pub hash_or_number: String,
}

impl BlockData {
    pub fn from_string(hash_or_number: String) -> Result<Self, BlockfrostError> {
        validate_hash_or_number(&hash_or_number).map_err(BlockfrostError::custom_400)?;

        Ok(Self {
            hash_or_number: hash_or_number.to_string(),
        })
    }
}

pub fn validate_hash_or_number(hash_or_number: &str) -> Result<(), String> {
    if is_number(hash_or_number) {
        if validate_positive_in_range_signed_int(hash_or_number) {
            Ok(())
        } else {
            Err("Missing, out of range or malformed block number.".to_string())
        }
    } else if validate_block_hash(hash_or_number) {
        Ok(())
    } else {
        Err("Missing or malformed block hash.".to_string())
    }
}

pub fn validate_positive_in_range_signed_int(possible_positive_int: &str) -> bool {
    if possible_positive_int.is_empty() {
        return false;
    }
    if let Ok(n) = possible_positive_int.parse::<i64>() {
        n > 0 && n <= MAX_SIGNED_INT
    } else {
        false
    }
}

pub fn is_number(value: &str) -> bool {
    !value.is_empty() && value.parse::<f64>().is_ok()
}

pub fn validate_block_hash(input: &str) -> bool {
    if input.len() != 64 {
        return false;
    }

    hex::decode(input).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::errors::BlockfrostError;
    use rstest::rstest;

    #[rstest]
    #[case("12345", Ok(()))]
    #[case("-1", Err("Missing, out of range or malformed block number.".to_string()))]
    #[case("2147483648", Err("Missing, out of range or malformed block number.".to_string()))]
    #[case("abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890", Ok(()))]
    #[case("invalid hex!", Err("Missing or malformed block hash.".to_string()))]
    #[case("", Err("Missing or malformed block hash.".to_string()))]
    fn test_validate_hash_or_number(#[case] input: &str, #[case] expected: Result<(), String>) {
        assert_eq!(validate_hash_or_number(input), expected);
    }

    #[rstest]
    #[case("12345", true)]
    #[case("0", false)]
    #[case("-1", false)]
    #[case("2147483648", false)]
    #[case("", false)]
    #[case("abc", false)]
    fn test_validate_positive_in_range_signed_int(#[case] input: &str, #[case] expected: bool) {
        assert_eq!(validate_positive_in_range_signed_int(input), expected);
    }

    #[rstest]
    #[case("12345", true)]
    #[case("123.45", true)]
    #[case("", false)]
    #[case("abc", false)]
    fn test_is_number(#[case] input: &str, #[case] expected: bool) {
        assert_eq!(is_number(input), expected);
    }

    #[rstest]
    #[case(
        "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
        true
    )]
    #[case("12345", false)]
    #[case("invalid hex!", false)]
    #[case("", false)]
    fn test_validate_block_hash(#[case] input: &str, #[case] expected: bool) {
        assert_eq!(validate_block_hash(input), expected);
    }

    #[rstest]
    #[case(
        "12345",
        Ok(BlockData { hash_or_number: "12345".to_string() })
    )]
    #[case(
        "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
        Ok(BlockData {
            hash_or_number: "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890".to_string()
        })
    )]
    #[case(
        "-1",
        Err(BlockfrostError::custom_400("Missing, out of range or malformed block number.".to_string()))
    )]
    #[case(
        "invalid hex!",
        Err(BlockfrostError::custom_400("Missing or malformed block hash.".to_string()))
    )]
    #[case(
        "",
        Err(BlockfrostError::custom_400("Missing or malformed block hash.".to_string()))
    )]
    fn test_from_string(#[case] input: &str, #[case] expected: Result<BlockData, BlockfrostError>) {
        assert_eq!(BlockData::from_string(input.to_string()), expected);
    }
}
