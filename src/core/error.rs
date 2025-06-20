use std::path::PathBuf;
use thiserror::Error;

/// Main error type for dotman-rs operations
#[derive(Error, Debug)]
pub enum DotmanError {
    /// I/O related errors
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Configuration related errors
    #[error("Configuration error: {0}")]
    Config(String),

    /// File system operation errors
    #[error("File system error: {message}")]
    FileSystem { message: String },

    /// Permission related errors
    #[error("Permission error: {message}")]
    Permission { message: String },

    /// Symlink related errors
    #[error("Symlink error: {message}")]
    Symlink { message: String },

    /// Backup operation errors
    #[error("Backup error: {message}")]
    Backup { message: String },

    /// Restore operation errors  
    #[error("Restore error: {message}")]
    Restore { message: String },

    /// Transaction related errors
    #[error("Transaction error: {message}")]
    Transaction { message: String },

    /// Privilege escalation errors
    #[error("Privilege error: {message}")]
    Privilege { message: String },

    /// Initialization errors
    #[error("Initialization error: {0}")]
    InitializationError(String),

    /// Path related errors
    #[error("Invalid path: {path}")]
    InvalidPath { path: PathBuf },

    /// File not found errors
    #[error("File not found: {path}")]
    FileNotFound { path: PathBuf },

    /// Directory not found errors
    #[error("Directory not found: {path}")]
    DirectoryNotFound { path: PathBuf },

    /// File already exists errors
    #[error("File already exists: {path}")]
    FileExists { path: PathBuf },

    /// Checksum mismatch errors
    #[error("Checksum mismatch for file: {path}")]
    ChecksumMismatch { path: PathBuf },

    /// Serialization/deserialization errors
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// JSON serialization errors
    #[error("JSON serialization error: {0}")]
    JsonSerialization(#[from] serde_json::Error),

    /// Shell expansion errors
    #[error("Shell expansion error: {0}")]
    ShellExpansion(#[from] shellexpand::LookupError<std::env::VarError>),

    /// Nix (Unix) system call errors
    #[error("Unix system error: {0}")]
    Nix(#[from] nix::Error),

    /// UUID parsing errors
    #[error("UUID error: {0}")]
    Uuid(#[from] uuid::Error),

    /// Time related errors
    #[error("Time error: {message}")]
    Time { message: String },

    /// Validation errors
    #[error("Validation error: {message}")]
    Validation { message: String },

    /// Conflict resolution errors
    #[error("Conflict resolution error: {message}")]
    Conflict { message: String },

    /// Generic operation errors with context
    #[error("Operation failed: {operation} - {message}")]
    Operation { operation: String, message: String },
}

/// Result type alias for dotman-rs operations
pub type Result<T> = std::result::Result<T, DotmanError>;

impl DotmanError {
    /// Create a configuration error
    pub fn config<S: Into<String>>(message: S) -> Self {
        Self::Config(message.into())
    }

    /// Create a file system error
    pub fn filesystem<S: Into<String>>(message: S) -> Self {
        Self::FileSystem { message: message.into() }
    }

    /// Create a permission error
    pub fn permission<S: Into<String>>(message: S) -> Self {
        Self::Permission { message: message.into() }
    }

    /// Create a symlink error
    pub fn symlink<S: Into<String>>(message: S) -> Self {
        Self::Symlink { message: message.into() }
    }

    /// Create a backup error
    pub fn backup<S: Into<String>>(message: S) -> Self {
        Self::Backup { message: message.into() }
    }

    /// Create a restore error
    pub fn restore<S: Into<String>>(message: S) -> Self {
        Self::Restore { message: message.into() }
    }

    /// Create a transaction error
    pub fn transaction<S: Into<String>>(message: S) -> Self {
        Self::Transaction { message: message.into() }
    }

    /// Create a privilege error
    pub fn privilege<S: Into<String>>(message: S) -> Self {
        Self::Privilege { message: message.into() }
    }

    /// Create a validation error
    pub fn validation<S: Into<String>>(message: S) -> Self {
        Self::Validation { message: message.into() }
    }

    /// Create a conflict error
    pub fn conflict<S: Into<String>>(message: S) -> Self {
        Self::Conflict { message: message.into() }
    }

    /// Create an operation error with context
    pub fn operation<S: Into<String>>(operation: S, message: S) -> Self {
        Self::Operation {
            operation: operation.into(),
            message: message.into(),
        }
    }

    /// Create a serialization error
    pub fn serialization<S: Into<String>>(message: S) -> Self {
        Self::Serialization(message.into())
    }

    /// Create an I/O error
    pub fn io<S: Into<String>>(message: S) -> Self {
        Self::Io(std::io::Error::new(std::io::ErrorKind::Other, message.into()))
    }

    /// Create a path error 
    pub fn path<S: Into<String>>(message: S) -> Self {
        Self::InvalidPath { path: PathBuf::from(message.into()) }
    }

    /// Create a file not found error
    pub fn file_not_found(path: PathBuf) -> Self {
        Self::FileNotFound { path }
    }

    /// Check if the error is related to permissions
    pub fn is_permission_related(&self) -> bool {
        matches!(self, Self::Permission { .. } | Self::Privilege { .. })
    }

    /// Check if the error is recoverable
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            Self::FileExists { .. } | Self::Conflict { .. } | Self::Validation { .. }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_error_creation() {
        let err = DotmanError::config("test config error");
        assert!(matches!(err, DotmanError::Config { .. }));

        let err = DotmanError::permission("test permission error");
        assert!(err.is_permission_related());

        let err = DotmanError::conflict("test conflict");
        assert!(err.is_recoverable());
    }

    #[test]
    fn test_path_errors() {
        let path = PathBuf::from("/test/path");
        let err = DotmanError::FileNotFound { path: path.clone() };
        
        match err {
            DotmanError::FileNotFound { path: p } => assert_eq!(p, path),
            _ => panic!("Wrong error type"),
        }
    }

    #[test]
    fn test_error_display() {
        let err = DotmanError::config("test message");
        let display_str = format!("{}", err);
        assert!(display_str.contains("Configuration error"));
        assert!(display_str.contains("test message"));
    }
} 