use crate::BlockfrostError;

use axum::Json;

pub mod accounts;
pub mod addresses;
pub mod assets;
pub mod blocks;
pub mod epochs;
pub mod governance;
pub mod health;
pub mod ledger;
pub mod metadata;
pub mod metrics;
pub mod network;
pub mod pools;
pub mod root;
pub mod scripts;
pub mod tx;
pub mod txs;
pub mod utils;

pub type ApiResult<T> = Result<Json<T>, BlockfrostError>;
