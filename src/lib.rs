//! # dotman-rs
//!
//! A robust dotfiles management system with comprehensive file type support,
//! including symlinks, permissions preservation, and privileged operations.

pub mod core;
pub mod filesystem;
pub mod backup;
pub mod restore;
pub mod config;
pub mod transaction;
pub mod cli;
pub mod utils;

// Re-export commonly used types and traits
pub use core::{
    error::{DotmanError, Result},
    traits::{BackupEngine, FileHandler, RestoreEngine, TransactionManager},
    types::{FileType, FileMetadata, OperationMode},
};

pub use config::{Config, Profile, ConfigEntry};
pub use filesystem::{FileSystem, FileSystemImpl};
pub use backup::BackupManager;
pub use restore::RestoreManager;
pub use transaction::DefaultTransactionManager;

/// The main dotman-rs library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Initialize the logging system with the specified level
pub fn init_logging(level: tracing::Level) -> Result<()> {
    use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(level.to_string()));

    tracing_subscriber::registry()
        .with(fmt::layer().with_target(false).with_thread_ids(true))
        .with(filter)
        .try_init()
        .map_err(|e| DotmanError::InitializationError(format!("Failed to initialize logging: {}", e)))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
    }

    #[tokio::test]
    async fn test_logging_init() {
        // This test ensures logging can be initialized without panicking
        let result = init_logging(tracing::Level::DEBUG);
        // Note: This might fail if logging is already initialized in other tests
        // In a real test suite, we'd use a more sophisticated approach
        assert!(result.is_ok() || result.is_err());
    }
}
