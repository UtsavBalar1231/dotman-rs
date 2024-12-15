use std::io;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Failed due to I/O error: {0}")]
    Io(#[from] io::Error),

    #[error("Failed due to formatting error: {0}")]
    Fmt(#[from] std::fmt::Error),

    #[error("Failed to serialize toml: {0}")]
    Serialization(#[from] toml::ser::Error),

    #[error("Failed to deserialize toml: {0}")]
    Deserialization(#[from] toml::de::Error),

    #[error("Failed due to invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("Failed due to invalid path: {0}")]
    InvalidPath(String),

    #[error("Failed due to unknown error: {0}")]
    Unknown(String),
}
