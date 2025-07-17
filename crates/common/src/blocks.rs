use serde::Deserialize;

#[derive(Deserialize)]
pub struct BlocksPath {
    pub hash_or_number: String,
}

pub struct BlocksSlotPath {
    pub slot: String,
}
