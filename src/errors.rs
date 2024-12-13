use std::io;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Failed due to I/O error: {0}")]
    Io(#[from] io::Error),

    #[error("Failed due to formatting error: {0}")]
    Fmt(#[from] std::fmt::Error),

    #[error("Failed to serialize ron: {0}")]
    Serialization(#[from] ron::Error),

    #[error("Failed to deserialize ron: {0}")]
    Deserialization(#[from] ron::de::SpannedError),

    #[error("Failed to serialize: {0}")]
    SerializationError(String),

    #[error("Failed due to invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("Failed due to unknown error: {0}")]
    Unknown(String),
}
