use serde::Deserialize;

#[derive(Deserialize)]
pub struct PoolsPath {
    pub pool_id: String,
}
