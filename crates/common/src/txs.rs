use serde::Deserialize;

#[derive(Deserialize)]
pub struct TxsPath {
    pub hash: String,
}
