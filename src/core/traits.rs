use std::path::{Path, PathBuf};
use async_trait::async_trait;
use uuid::Uuid;

use crate::core::{
    error::Result,
    types::{FileMetadata, OperationResult, ProgressInfo},
};

/// Trait for handling different file types
#[async_trait]
pub trait FileHandler: Send + Sync {
    /// Get metadata for a file
    async fn get_metadata(&self, path: &Path) -> Result<FileMetadata>;

    /// Copy a file from source to destination
    async fn copy(&self, src: &Path, dst: &Path, metadata: &FileMetadata) -> Result<OperationResult>;

    /// Verify that a file matches its expected metadata
    async fn verify(&self, path: &Path, expected: &FileMetadata) -> Result<bool>;

    /// Check if this handler can handle the given file type
    fn can_handle(&self, metadata: &FileMetadata) -> bool;

    /// Get the priority of this handler (higher priority handlers are preferred)
    fn priority(&self) -> u32;
}

/// Main trait for backup operations
#[async_trait]
pub trait BackupEngine: Send + Sync {
    /// Perform a backup operation for multiple files
    async fn backup_files(&self, source_paths: Vec<PathBuf>) -> Result<Vec<OperationResult>>;

    /// Verify backup integrity
    async fn verify_backup(&self, backup_path: &Path) -> Result<bool>;
}

/// Main trait for restore operations
#[async_trait]
pub trait RestoreEngine: Send + Sync {
    /// Perform a restore operation for multiple files
    async fn restore_files(&self, backup_path: PathBuf, target_paths: Vec<PathBuf>) -> Result<Vec<OperationResult>>;

    /// Verify restore integrity
    async fn verify_restore(&self, target_paths: &[PathBuf]) -> Result<bool>;

    /// List backup contents
    async fn list_backup_contents(&self, backup_path: &Path) -> Result<Vec<FileMetadata>>;
}

/// Trait for managing transactions and rollback
#[async_trait]
pub trait TransactionManager: Send + Sync {
    /// Start a new transaction
    async fn begin_transaction(&self) -> Result<Uuid>;

    /// Commit a transaction
    async fn commit_transaction(&self, transaction_id: Uuid) -> Result<()>;

    /// Rollback a transaction
    async fn rollback_transaction(&self, transaction_id: Uuid) -> Result<()>;

    /// Add an operation to a transaction
    async fn add_operation(
        &self,
        transaction_id: Uuid,
        operation: OperationResult,
    ) -> Result<()>;

    /// Get the status of a transaction
    async fn get_transaction_status(&self, transaction_id: Uuid) -> Result<TransactionStatus>;

    /// List all active transactions
    async fn list_active_transactions(&self) -> Result<Vec<Uuid>>;
}

/// Status of a transaction
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransactionStatus {
    /// Transaction is active and can accept operations
    Active,
    /// Transaction is being committed
    Committing,
    /// Transaction has been committed successfully
    Committed,
    /// Transaction is being rolled back
    RollingBack,
    /// Transaction has been rolled back
    RolledBack,
    /// Transaction failed and cannot be recovered
    Failed,
}

/// Trait for reporting progress of long-running operations
pub trait ProgressReporter: Send + Sync {
    /// Report progress information
    fn report_progress(&self, progress: &ProgressInfo);
}

/// Trait for privilege escalation
#[async_trait]
pub trait PrivilegeManager: Send + Sync {
    /// Check if elevated privileges are available
    async fn has_elevated_privileges(&self) -> bool;

    /// Request elevated privileges for an operation
    async fn request_privileges(&self, reason: &str) -> Result<()>;

    /// Execute an operation with elevated privileges
    async fn execute_with_privileges<F, T>(&self, operation: F) -> Result<T>
    where
        F: FnOnce() -> Result<T> + Send,
        T: Send;

    /// Check if a path requires elevated privileges
    async fn requires_privileges(&self, path: &Path) -> bool;
}

/// Trait for file system abstraction
#[async_trait]
pub trait FileSystem: Send + Sync {
    /// Check if a path exists
    async fn exists(&self, path: &Path) -> Result<bool>;

    /// Create a directory and all parent directories
    async fn create_dir_all(&self, path: &Path) -> Result<()>;

    /// Remove a file or directory
    async fn remove(&self, path: &Path) -> Result<()>;

    /// Copy a file or directory
    async fn copy(&self, src: &Path, dst: &Path) -> Result<()>;

    /// Move/rename a file or directory
    async fn move_file(&self, src: &Path, dst: &Path) -> Result<()>;

    /// Get file metadata
    async fn metadata(&self, path: &Path) -> Result<FileMetadata>;

    /// List directory contents
    async fn list_dir(&self, path: &Path) -> Result<Vec<FileMetadata>>;

    /// Read file contents
    async fn read_file(&self, path: &Path) -> Result<Vec<u8>>;

    /// Write file contents
    async fn write_file(&self, path: &Path, contents: &[u8]) -> Result<()>;

    /// Create a symbolic link
    async fn create_symlink(&self, target: &Path, link: &Path) -> Result<()>;

    /// Read a symbolic link target
    async fn read_symlink(&self, path: &Path) -> Result<std::path::PathBuf>;

    /// Copy a single file (not directory)
    async fn copy_file(&self, src: &Path, dst: &Path) -> Result<()>;

    /// Remove a single file (not directory)
    async fn remove_file(&self, path: &Path) -> Result<()>;
}

/// Trait for configuration management
pub trait ConfigManager: Send + Sync {
    /// Load configuration from a file
    fn load_config(&self, path: &Path) -> Result<crate::config::Config>;

    /// Save configuration to a file
    fn save_config(&self, config: &crate::config::Config, path: &Path) -> Result<()>;

    /// Validate configuration
    fn validate_config(&self, config: &crate::config::Config) -> Result<()>;

    /// Migrate configuration from older versions
    fn migrate_config(&self, config: &mut crate::config::Config) -> Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transaction_status() {
        let status = TransactionStatus::Active;
        assert_eq!(status, TransactionStatus::Active);
        assert_ne!(status, TransactionStatus::Committed);
    }

    // Note: Most trait testing would be done with mock implementations
    // in integration tests, as these are primarily interface definitions
} 