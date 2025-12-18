use clap::ValueEnum;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, ValueEnum, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Network {
    Mainnet,
    Preprod,
    Preview,
}

impl Network {
    pub fn network_magic(&self) -> u64 {
        match self {
            Self::Mainnet => 764824073,
            Self::Preprod => 1,
            Self::Preview => 2,
        }
    }

    pub fn is_testnet(&self) -> bool {
        *self != Self::Mainnet
    }
}
