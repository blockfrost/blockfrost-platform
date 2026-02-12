use serde::Deserialize;

fn default_version() -> String {
    "5".to_string()
}

#[derive(Deserialize)]
pub struct EvaluateQuery {
    // The default version is 5, which represents Ogmios v5.
    #[serde(default = "default_version")]
    pub version: String,
    pub evaluator: Option<String>,
}
