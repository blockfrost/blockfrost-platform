use crate::{errors::BlockfrostError, pagination::Order, types::Amount};
use serde::Deserialize;

const POLICY_ID_SIZE: usize = 56;

pub struct AssetData {
    pub asset: String,
}

#[derive(Deserialize)]
pub struct AssetsPath {
    pub asset: String,
}

impl AssetData {
    pub fn from_query(asset: String) -> Result<Self, BlockfrostError> {
        let is_valid = validate_asset_name(&asset);

        if !is_valid {
            return Err(BlockfrostError::invalid_asset_name());
        }

        Ok(AssetData { asset })
    }
}

pub struct ParsedAsset {
    pub policy_id: String,
    pub asset_name_hex: String,
}

pub fn validate_asset_name(asset_name: &str) -> bool {
    if asset_name == "lovelace" {
        return true;
    }

    if hex::decode(asset_name).is_ok() {
        return asset_name.len() >= 56 && asset_name.len() <= 120;
    }

    false
}

pub fn parse_asset(hex: &str) -> Result<ParsedAsset, BlockfrostError> {
    if hex.len() < POLICY_ID_SIZE {
        return Err(BlockfrostError::internal_server_error(format!(
            "Asset name is too short: {}",
            hex,
        )));
    }

    let (policy_id, asset_name_in_hex) = hex.split_at(POLICY_ID_SIZE);

    Ok(ParsedAsset {
        policy_id: policy_id.to_string(),
        asset_name_hex: asset_name_in_hex.to_string(),
    })
}

pub fn sort_asset_array(amount: &mut [Amount], order: &Order) {
    amount.sort_by(|a, b| {
        let is_lovelace_a = a.unit == "lovelace";
        let is_lovelace_b = b.unit == "lovelace";

        if is_lovelace_a && !is_lovelace_b {
            return std::cmp::Ordering::Less;
        } else if !is_lovelace_a && is_lovelace_b {
            return std::cmp::Ordering::Greater;
        }

        let asset_a = parse_asset(&a.unit);
        let asset_b = parse_asset(&b.unit);

        let result = match (asset_a, asset_b) {
            (Ok(ref a), Ok(ref b)) => match a.policy_id.cmp(&b.policy_id) {
                std::cmp::Ordering::Equal => a.asset_name_hex.cmp(&b.asset_name_hex),
                other => other,
            },
            (Err(ref err), Ok(_)) | (Err(ref err), Err(_)) => {
                sentry::capture_message(
                    &format!("Failed to parse asset from kupo: {}", err),
                    sentry::Level::Error,
                );
                std::cmp::Ordering::Less
            },
            (Ok(_), Err(ref err)) => {
                sentry::capture_message(
                    &format!("Failed to parse asset from kupo: {}", err),
                    sentry::Level::Error,
                );
                std::cmp::Ordering::Less
            },
        };

        if order == &Order::Asc {
            result
        } else {
            result.reverse()
        }
    });
}

#[cfg(test)]
mod tests {
    use crate::assets::sort_asset_array;
    use crate::pagination::Order;
    use crate::types::Amount;
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    #[rstest]
    #[case(
        vec![
            Amount { unit: "43b07d4037f0d75ee10f9863097463fc02ff3c0b8b705ae61d9c75bf4d796e746820546f6b656e".to_string(), quantity: "100000000".to_string() },
            Amount { unit: "08745cbfeed4d42985b9fb6accd955514e21d9425e6746268c46360c5455524638345752464d36563758".to_string(), quantity: "1".to_string() },
            Amount { unit: "22aae60dc7877bca8be2fb82ede807747d0b207cda51adf519853430756e64657266756e646564676f6c6430303037".to_string(), quantity: "1".to_string() },
        ], vec![
            Amount { unit: "08745cbfeed4d42985b9fb6accd955514e21d9425e6746268c46360c5455524638345752464d36563758".to_string(), quantity: "1".to_string() },
            Amount { unit: "22aae60dc7877bca8be2fb82ede807747d0b207cda51adf519853430756e64657266756e646564676f6c6430303037".to_string(), quantity: "1".to_string() },
            Amount { unit: "43b07d4037f0d75ee10f9863097463fc02ff3c0b8b705ae61d9c75bf4d796e746820546f6b656e".to_string(), quantity: "100000000".to_string() },
        ], Order::Asc)]
    #[case(
        vec![
            Amount { unit: "da8c30857834c6ae7203935b89278c532b3995245295456f993e1d244c51".to_string(), quantity: "3487087727563".to_string() },
            Amount { unit: "d436d9f6b754582f798fe33f4bed12133d47493f78b944b9cc55fd1853756d6d69744c6f6467653138343232".to_string(), quantity: "1".to_string() },
            Amount { unit: "d436d9f6b754582f798fe33f4bed12133d47493f78b944b9cc55fd1853756d6d69744c6f6467653131373138".to_string(), quantity: "1".to_string() },
        ], vec![
            Amount { unit: "da8c30857834c6ae7203935b89278c532b3995245295456f993e1d244c51".to_string(), quantity: "3487087727563".to_string() },
            Amount { unit: "d436d9f6b754582f798fe33f4bed12133d47493f78b944b9cc55fd1853756d6d69744c6f6467653138343232".to_string(), quantity: "1".to_string() },
            Amount { unit: "d436d9f6b754582f798fe33f4bed12133d47493f78b944b9cc55fd1853756d6d69744c6f6467653131373138".to_string(), quantity: "1".to_string() },
        ],  Order::Desc)]
    fn test_sort_asset_array(
        #[case] mut input: Vec<Amount>,
        #[case] expected: Vec<Amount>,
        #[case] order: Order,
    ) {
        sort_asset_array(&mut input, &order);
        assert_eq!(input, expected);
    }

    #[rstest]
    #[case(
        "Valid asset (min length)",
        "00000002df633853f6a47465c9496721d2d5b1291b8398016c0e87ae",
        true
    )]
    #[case(
        "Valid asset (in between length)",
        "00000002df633853f6a47465c9496721d2d5b1291b8398016c0e87ae6e7574636f696e",
        true
    )]
    #[case(
        "Valid asset (max length)",
        "fc373a6cfc24c11d925dc48535f661d54edbb04646bea645e7d58ee0447261676f6e73496e6665726e6f516d516d446d357337694376397136653569",
        true
    )]
    #[case(
        "Invalid asset ( < length)",
        "00000002df633853f6a47465c9496721d2d5b1291b8398016c0e87a",
        false
    )]
    #[case(
        "Invalid asset ( > length)",
        "fc373a6cfc24c11d925dc48535f661d54edbb04646bea645e7d58ee0447261676f6e73496e6665726e6f516d516d446d3573376943763971366535699",
        false
    )]
    #[case(
        "Invalid asset (hex)",
        "00000002df633853f6a47465c9496721d2d5b1291b8398016c0e87ae6e7574636f696eg",
        false
    )]
    #[case(
        "Invalid asset ( < length & hex)",
        "00000002df633853f6a47465c9496721d2d5b1291b8398016c0e87g",
        false
    )]
    #[case("lovelace asset", "lovelace", true)]
    #[case(
        "Invalid asset ( > length & hex)",
        "fc373a6cfc24c11d925dc48535f661d54edbb04646bea645e7d58ee0447261676f6e73496e6665726e6f516d516d446d357337694376397136653569g",
        false
    )]
    fn test_validate_asset(#[case] description: &str, #[case] input: &str, #[case] expected: bool) {
        assert_eq!(
            crate::assets::validate_asset_name(input),
            expected,
            "{}",
            description
        );
    }
}
