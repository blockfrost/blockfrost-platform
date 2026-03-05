use serde::Deserialize;

fn default_version() -> u8 {
    5
}

#[derive(Deserialize)]
pub struct EvaluateQuery {
    // The default version is 5, which represents Ogmios v5.
    #[serde(default = "default_version")]
    pub version: u8,
}
