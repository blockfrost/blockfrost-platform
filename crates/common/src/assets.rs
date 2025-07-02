use crate::errors::BlockfrostError;
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
            "Asset name is too short: {hex}",
        )));
    }

    let (policy_id, asset_name_in_hex) = hex.split_at(POLICY_ID_SIZE);

    Ok(ParsedAsset {
        policy_id: policy_id.to_string(),
        asset_name_hex: asset_name_in_hex.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use rstest::rstest;

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
