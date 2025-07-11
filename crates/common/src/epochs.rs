use serde::Deserialize;

#[derive(Deserialize)]
pub struct EpochsPath {
    pub epoch_number: String,
}
