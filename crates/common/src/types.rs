use crate::errors::BlockfrostError;
use axum::Json;
use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use tracing::Level;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Amount {
    pub unit: String,
    pub quantity: String,
}

#[derive(Debug, Clone, ValueEnum, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Network {
    Mainnet,
    Preprod,
    Preview,
    Custom,
}

impl Network {
    // FIXME: use serde? But it allocs
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Mainnet => "mainnet",
            Self::Preprod => "preprod",
            Self::Preview => "preview",
            Self::Custom => "custom",
        }
    }
}

#[derive(Debug, Clone, ValueEnum, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
    Trace,
}

// Implement conversion from LogLevel enum to tracing::Level
impl From<LogLevel> for Level {
    fn from(log_level: LogLevel) -> Self {
        match log_level {
            LogLevel::Debug => Level::DEBUG,
            LogLevel::Info => Level::INFO,
            LogLevel::Warn => Level::WARN,
            LogLevel::Error => Level::ERROR,
            LogLevel::Trace => Level::TRACE,
        }
    }
}

pub type ApiResult<T> = Result<Json<T>, BlockfrostError>;
