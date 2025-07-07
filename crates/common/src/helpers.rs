use crate::errors::BlockfrostError;
use pallas_network::miniprotocols::localstate::queries_v16::BigInt;

/// This function allows us to take both hex-encoded and raw bytes. It has
/// to be a heuristic: if there are input bytes that are not `[0-9a-f]`,
/// then it must be a binary string. Otherwise, we assume it’s hex encoded.
///
/// **Note**: there is a small probability that the user gave us a binary
/// string that only _looked_ like a hex-encoded one, but it’s rare enough
/// to ignore it.
pub fn binary_or_hex_heuristic(xs: &[u8]) -> Vec<u8> {
    let even_length = xs.len() % 2 == 0;
    let contains_non_hex = xs.iter().any(|&x| !x.is_ascii_hexdigit());

    if !even_length || contains_non_hex {
        xs.to_vec()
    } else {
        hex::decode(xs).unwrap_or_else(|_| unreachable!())
    }
}

pub fn convert_bigint(bigint: BigInt) -> Result<i128, BlockfrostError> {
    match bigint {
        BigInt::Int(big) => Ok(i128::from(big)),
        _ => Err(BlockfrostError::internal_server_error(
            "Invalid/unsupported BigInt format".to_string(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn proptest_binary_or_hex_heuristic(
            binary in prop::collection::vec(any::<u8>(), 0..=128)
                .prop_filter("exclude values made up only of hex digits", |xs| {
                    let contains_non_hex = xs.iter().any(|&x| !x.is_ascii_hexdigit());
                    contains_non_hex
                })
        ) {
            let hex_string = hex::encode(&binary);
            assert_eq!(
                binary_or_hex_heuristic(hex_string.as_bytes()),
                binary_or_hex_heuristic(&binary)
            )
        }
    }
}
