use crate::errors::BlockfrostError;
use bech32::{Bech32, Hrp};
use serde::Deserialize;

const SPECIAL_DREP_IDS: &[&str] = &["drep_always_abstain", "drep_always_no_confidence"];

#[derive(Deserialize)]
pub struct DrepsPath {
    pub drep_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DRepData {
    pub drep_id: String,
}

impl DRepData {
    pub fn new(drep_id: String) -> Result<DRepData, BlockfrostError> {
        DRepData { drep_id }.to_cip129()
    }

    pub fn to_cip129(&self) -> Result<DRepData, BlockfrostError> {
        if SPECIAL_DREP_IDS.contains(&self.drep_id.as_str()) {
            return Ok(DRepData {
                drep_id: self.drep_id.clone(),
            });
        }

        let (hrp, raw_bytes) = bech32::decode(&self.drep_id)?;

        let is_script = match hrp.as_str() {
            "drep" => false,
            "drep_script" => true,
            _ => {
                return Err(BlockfrostError::internal_server_error(
                    "Invalid drep id prefix".into(),
                ));
            },
        };

        match raw_bytes.len() {
            28 => {
                // Legacy format (CIP-105)
                let key_type_nibble = 0x2 << 4; // DRep
                let credential_type_nibble = if is_script { 0x3 } else { 0x2 }; // 3 = ScriptHash, 2 = KeyHash
                let header = key_type_nibble | credential_type_nibble;

                let mut bytes_with_header = vec![header];
                bytes_with_header.extend_from_slice(&raw_bytes);

                let hrp = Hrp::parse("drep")?;
                let cip129_id = bech32::encode::<Bech32>(hrp, &bytes_with_header)?;

                Ok(DRepData { drep_id: cip129_id })
            },
            29 => {
                // Already CIP-129 (Header 29)
                Ok(DRepData {
                    drep_id: self.drep_id.clone(),
                })
            },
            _ => Err(BlockfrostError::internal_server_error(
                "Invalid DRep ID length".into(),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    struct DRepFixture {
        input: &'static str,
        expected_id: &'static str,
    }

    #[rstest]
    #[case::always_abstain(DRepFixture {
        input: "drep_always_abstain",
        expected_id: "drep_always_abstain",
    })]
    #[case::always_no_confidence(DRepFixture {
        input: "drep_always_no_confidence",
        expected_id: "drep_always_no_confidence",
    })]
    #[case::regular_key_hash(DRepFixture {
        input: "drep1y3wylkrkyt3q6u078ajh8f2henflpsq5hrcqhfa3yfmlqx7z66n",
        expected_id: "drep1ygj9cn7cwc3wyrt3lclk2ua92lxd8uxqzju0qza8ky380uqjnj28h",
    })]
    #[case::regular_key_hash2(DRepFixture {
        input: "drep1edu7a90eszdus0hguck2w3lxr5r0juvc9frrxv3d2e6fcnqte0e",
        expected_id: "drep1yt9hnm54lxqfhjp7arnzef68ucwsd7t3nq4yvvej94t8f8qgam3dv",
    })]
    #[case::script_hash(DRepFixture {
        input: "drep_script1hmgwyt6zv89j5htlnwcttk95lr0x7r87sxzr9dumxnc3vadhlap",
        expected_id: "drep1ywldpc30gfsuk2ja07dmpdwcknudumcvl6qcgv4hnv60z9sl4umuv",
    })]
    #[case::regular_hash_no_script(DRepFixture {
        input: "drep1hmgwyt6zv89j5htlnwcttk95lr0x7r87sxzr9dumxnc3vj02hpq",
        expected_id: "drep1y2ldpc30gfsuk2ja07dmpdwcknudumcvl6qcgv4hnv60z9sl8v2ut",
    })]
    #[case::script_hash_another(DRepFixture {
        input: "drep_script16pxnn38ykshfahwmkaqmke3kdqaksg4w935d7uztvh8y5sh6f6d",
        expected_id: "drep1y0gy6wwyuj6za8kamwm5rwmxxe5rk6pz4ckx3hmsfdjuujsr70shz",
    })]
    #[case::cip129_true_case(DRepFixture {
        input: "drep1y0gy6wwyuj6za8kamwm5rwmxxe5rk6pz4ckx3hmsfdjuujsr70shz",
        expected_id: "drep1y0gy6wwyuj6za8kamwm5rwmxxe5rk6pz4ckx3hmsfdjuujsr70shz",
    })]
    #[case::cip129_test_vector(DRepFixture {
        input: "drep1ygqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq7vlc9n",
        expected_id: "drep1ygqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq7vlc9n",
    })]
    #[case::hosky(DRepFixture {
        input: "drep1yf2jzhuc4f7eu2yay9d9ta3dykxxcwn34wz8kak7nhd7vcgrxn7ns",
        expected_id: "drep1yf2jzhuc4f7eu2yay9d9ta3dykxxcwn34wz8kak7nhd7vcgrxn7ns",
    })]
    #[case::sebastien(DRepFixture {
        input: "drep1y2csyxt7u2hl4674pl9cef5lknafaw5nraxvyx033kmd0es3awuv0",
        expected_id: "drep1y2csyxt7u2hl4674pl9cef5lknafaw5nraxvyx033kmd0es3awuv0",
    })]

    fn test_to_cip129(#[case] fixture: DRepFixture) {
        let drep = DRepData {
            drep_id: fixture.input.to_string(),
        };

        let drep = drep.to_cip129().expect("Conversion failed");
        assert_eq!(drep.drep_id, fixture.expected_id);
    }
}
