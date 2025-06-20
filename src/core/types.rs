use std::path::PathBuf;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::path::Path;

/// Represents different types of files in the system
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileType {
    /// Regular file
    File,
    /// Directory
    Directory,
    /// Symbolic link
    Symlink {
        /// Target of the symlink
        target: PathBuf,
        /// Whether the symlink is absolute or relative
        is_absolute: bool,
        /// Whether the target exists
        target_exists: bool,
    },
    /// Character device
    CharDevice,
    /// Block device
    BlockDevice,
    /// Named pipe (FIFO)
    Fifo,
    /// Unix domain socket
    Socket,
    /// Unknown or unsupported file type
    Unknown,
}

/// Comprehensive file metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    /// File path
    pub path: PathBuf,
    /// File type
    pub file_type: FileType,
    /// File size in bytes
    pub size: u64,
    /// File permissions (Unix mode)
    pub permissions: u32,
    /// Owner user ID
    pub uid: u32,
    /// Owner group ID
    pub gid: u32,
    /// Last modified time
    pub modified: DateTime<Utc>,
    /// Last accessed time
    pub accessed: DateTime<Utc>,
    /// Creation time (if available)
    pub created: Option<DateTime<Utc>>,
    /// File content hash (for files)
    pub content_hash: Option<String>,
    /// Directory content hash (for directories)
    pub directory_hash: Option<String>,
    /// Extended attributes
    pub extended_attributes: HashMap<String, Vec<u8>>,
    /// Whether the file requires elevated privileges to access
    pub requires_privileges: bool,
}

/// Operation mode for dotman operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OperationMode {
    /// Default mode with standard behavior
    Default,
    /// Preview mode - don't make actual changes, just show what would happen
    Preview,
    /// Normal mode - standard operation with prompts for conflicts
    Normal,
    /// Force mode - overwrite existing files without prompting
    Force,
    /// Interactive mode - prompt for every operation
    Interactive,
}

/// Result of a file operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationResult {
    /// Type of operation performed
    pub operation_type: OperationType,
    /// Path that was operated on
    pub path: PathBuf,
    /// Whether the operation succeeded
    pub success: bool,
    /// Optional error message if operation failed
    pub error: Option<String>,
    /// Operation details or additional information
    pub details: Option<String>,
    /// Privileges required for this operation
    pub required_privileges: bool,
    /// Duration of the operation
    pub duration: Option<std::time::Duration>,
    /// Bytes processed during the operation
    pub bytes_processed: Option<u64>,
}

/// Types of operations that can be performed
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OperationType {
    /// Backup operation
    Backup,
    /// Restore operation
    Restore,
    /// Copy operation
    Copy,
    /// Move operation
    Move,
    /// Delete operation
    Delete,
    /// Create symlink operation
    CreateSymlink,
    /// Verify operation
    Verify,
    /// Clean operation
    Clean,
}

/// Represents a conflict between backup and current state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conflict {
    /// Path where the conflict occurred
    pub path: PathBuf,
    /// Type of conflict
    pub conflict_type: ConflictType,
    /// Metadata of the backup version
    pub backup_metadata: FileMetadata,
    /// Metadata of the current version
    pub current_metadata: FileMetadata,
    /// Suggested resolution
    pub suggested_resolution: ConflictResolution,
    /// Current resolution being applied
    pub resolution: Option<ConflictResolution>,
}

/// Types of conflicts that can occur
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConflictType {
    /// File exists in both backup and current location with different content
    ContentMismatch,
    /// File type differs between backup and current
    TypeMismatch,
    /// Permissions differ between backup and current
    PermissionMismatch,
    /// File exists in current location but not in backup
    UnexpectedFile,
    /// File exists in backup but current location is occupied by different file
    PathOccupied,
    /// Symlink target differs between backup and current
    SymlinkTargetMismatch,
}

/// Possible resolutions for conflicts
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConflictResolution {
    /// Keep the backup version
    KeepBackup,
    /// Keep the current version
    KeepCurrent,
    /// Merge the changes (if possible)
    Merge,
    /// Create a backup of current and restore from backup
    BackupAndRestore,
    /// Skip this file
    Skip,
    /// Prompt user for decision
    AskUser,
    /// Ask user for decision (alias for AskUser)
    Ask,
}

/// Progress information for long-running operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressInfo {
    /// Current progress count
    pub current: u64,
    /// Total number of items to process
    pub total: u64,
    /// Progress message
    pub message: String,
    /// Additional details
    pub details: Option<String>,
}

impl ProgressInfo {
    /// Create a new progress info
    pub fn new(current: u64, total: u64, message: String) -> Self {
        Self {
            current,
            total,
            message,
            details: None,
        }
    }

    /// Calculate completion percentage
    pub fn percentage(&self) -> f64 {
        if self.total == 0 {
            100.0
        } else {
            (self.current as f64 / self.total as f64) * 100.0
        }
    }

    /// Check if the operation is complete
    pub fn is_complete(&self) -> bool {
        self.current >= self.total
    }
}

impl FileMetadata {
    /// Create a new FileMetadata instance
    pub fn new(path: PathBuf, file_type: FileType) -> Self {
        let now = Utc::now();
        Self {
            path,
            file_type,
            size: 0,
            permissions: 0o644,
            uid: 0,
            gid: 0,
            modified: now,
            accessed: now,
            created: Some(now),
            content_hash: None,
            directory_hash: None,
            extended_attributes: HashMap::new(),
            requires_privileges: false,
        }
    }

    /// Check if this file is a symlink
    pub fn is_symlink(&self) -> bool {
        matches!(self.file_type, FileType::Symlink { .. })
    }

    /// Check if this file is a directory
    pub fn is_directory(&self) -> bool {
        matches!(self.file_type, FileType::Directory)
    }

    /// Check if this file is a regular file
    pub fn is_file(&self) -> bool {
        matches!(self.file_type, FileType::File)
    }

    /// Get the symlink target if this is a symlink
    pub fn symlink_target(&self) -> Option<&PathBuf> {
        match &self.file_type {
            FileType::Symlink { target, .. } => Some(target),
            _ => None,
        }
    }
}

impl OperationResult {
    /// Create a successful operation result
    pub fn success(operation_type: OperationType, path: PathBuf) -> Self {
        Self {
            operation_type,
            path,
            success: true,
            error: None,
            details: None,
            required_privileges: false,
            duration: None,
            bytes_processed: None,
        }
    }

    /// Create a failed operation result
    pub fn failure(operation_type: OperationType, path: PathBuf, error: String) -> Self {
        Self {
            operation_type,
            path,
            success: false,
            error: Some(error),
            details: None,
            required_privileges: false,
            duration: None,
            bytes_processed: None,
        }
    }

    /// Create an operation result with details
    pub fn with_details(
        operation_type: OperationType, 
        path: PathBuf, 
        success: bool, 
        details: String
    ) -> Self {
        Self {
            operation_type,
            path,
            success,
            error: None,
            details: Some(details),
            required_privileges: false,
            duration: None,
            bytes_processed: None,
        }
    }
}

impl AsRef<Path> for FileMetadata {
    fn as_ref(&self) -> &Path {
        &self.path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_file_metadata_creation() {
        let path = PathBuf::from("/test/file.txt");
        let metadata = FileMetadata::new(path.clone(), FileType::File);
        
        assert_eq!(metadata.path, path);
        assert!(metadata.is_file());
        assert!(!metadata.is_directory());
        assert!(!metadata.is_symlink());
    }

    #[test]
    fn test_symlink_metadata() {
        let path = PathBuf::from("/test/link");
        let target = PathBuf::from("/test/target");
        let file_type = FileType::Symlink {
            target: target.clone(),
            is_absolute: true,
            target_exists: true,
        };
        
        let metadata = FileMetadata::new(path, file_type);
        
        assert!(metadata.is_symlink());
        assert_eq!(metadata.symlink_target(), Some(&target));
    }

    #[test]
    fn test_progress_info() {
        let progress = ProgressInfo::new(0, 100, "Starting".to_string());
        assert_eq!(progress.percentage(), 0.0);
        assert!(!progress.is_complete());
        
        let progress = ProgressInfo::new(50, 100, "Half done".to_string());
        assert_eq!(progress.percentage(), 50.0);
        
        let progress = ProgressInfo::new(100, 100, "Complete".to_string());
        assert!(progress.is_complete());
        assert_eq!(progress.percentage(), 100.0);
    }

    #[test]
    fn test_operation_result() {
        let result = OperationResult {
            path: PathBuf::from("/test/file"),
            operation_type: OperationType::Backup,
            success: true,
            error: None,
            details: Some("File backed up successfully".to_string()),
            required_privileges: false,
            duration: None,
            bytes_processed: None,
        };
        
        assert!(result.success);
        assert!(result.error.is_none());
        assert!(result.details.is_some());
    }
} 