use crate::errors::BlockfrostError;
use pallas_network::miniprotocols::localstate::queries_v16::{BigInt, SystemStart};

/// This function allows us to take both hex-encoded and raw bytes. It has
/// to be a heuristic: if there are input bytes that are not `[0-9a-f]`,
/// then it must be a binary string. Otherwise, we assume it’s hex encoded.
///
/// **Note**: there is a small probability that the user gave us a binary
/// string that only _looked_ like a hex-encoded one, but it’s rare enough
/// to ignore it.
pub fn binary_or_hex_heuristic(xs: &[u8]) -> Vec<u8> {
    let even_length = xs.len().is_multiple_of(2);
    let contains_non_hex = xs.iter().any(|&x| !x.is_ascii_hexdigit());

    if !even_length || contains_non_hex {
        xs.to_vec()
    } else {
        hex::decode(xs).unwrap_or_else(|_| unreachable!())
    }
}

pub fn convert_bigint(bigint: &BigInt) -> Result<i128, BlockfrostError> {
    match bigint {
        BigInt::Int(big) => Ok(i128::from(*big)),
        _ => Err(BlockfrostError::internal_server_error(
            "Invalid/unsupported BigInt format".to_string(),
        )),
    }
}

/// Convert a `SystemStart` to Unix epoch milliseconds.
///
/// Date arithmetic is done manually to avoid adding a `chrono` dependency to `common`.
///
/// # Panics
/// Panics if `SystemStart` contains an unsupported `BigInt` variant.
/// This should only be called during startup with data from the Cardano node.
pub fn system_start_to_epoch_millis(system_start: &SystemStart) -> u64 {
    let year = i64::try_from(
        convert_bigint(&system_start.year).expect("Failed to convert SystemStart year"),
    )
    .expect("SystemStart year out of i64 range");
    let day_of_year = system_start.day_of_year;
    let picos = i64::try_from(
        convert_bigint(&system_start.picoseconds_of_day)
            .expect("Failed to convert SystemStart picoseconds"),
    )
    .expect("SystemStart picoseconds out of i64 range");

    let epoch_days = days_from_year(year) + day_of_year - 1;
    (epoch_days as u64) * 86_400_000 + (picos / 1_000_000_000) as u64
}

/// Days from Unix epoch (1970-01-01) to January 1 of the given year.
fn days_from_year(year: i64) -> i64 {
    let y = year - 1;
    365 * (year - 1970) + (y / 4 - 492) - (y / 100 - 19) + (y / 400 - 4)
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
                    xs.iter().any(|&x| !x.is_ascii_hexdigit())
                })
        ) {
            let hex_string = hex::encode(&binary);
            assert_eq!(
                binary_or_hex_heuristic(hex_string.as_bytes()),
                binary_or_hex_heuristic(&binary)
            )
        }
    }

    #[test]
    fn test_days_from_year_known_epochs() {
        // 1970-01-01 is day 0
        assert_eq!(days_from_year(1970), 0);
        // 2000-01-01 = 10957 days after epoch
        assert_eq!(days_from_year(2000), 10957);
        // 2017-01-01 = 17167 days after epoch (Cardano mainnet era)
        assert_eq!(days_from_year(2017), 17167);
    }

    #[test]
    fn test_system_start_to_epoch_millis_mainnet() {
        // Cardano mainnet SystemStart: 2017-09-23T21:44:51Z
        // Unix timestamp: 1506203091000 ms
        let system_start = SystemStart {
            year: BigInt::Int(2017.into()),
            day_of_year: 266, // September 23 = day 266
            picoseconds_of_day: BigInt::Int(
                78_291_000_000_000_000i64.into(), // 21:44:51 = 78291s = 78291e12 picos
            ),
        };
        assert_eq!(system_start_to_epoch_millis(&system_start), 1506203091000);
    }

    #[test]
    fn test_system_start_to_epoch_millis_preview() {
        // Cardano preview SystemStart: 2022-11-01T00:00:00Z
        // Unix timestamp: 1667260800000 ms
        let system_start = SystemStart {
            year: BigInt::Int(2022.into()),
            day_of_year: 305, // November 1 = day 305
            picoseconds_of_day: BigInt::Int(0.into()),
        };
        assert_eq!(system_start_to_epoch_millis(&system_start), 1667260800000);
    }
}
