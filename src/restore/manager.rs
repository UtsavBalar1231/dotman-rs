use std::path::{Path, PathBuf};
use async_trait::async_trait;
use uuid::Uuid;
use chrono::Utc;
use tracing::{info, warn, error, debug, instrument};

use crate::core::{
    error::{DotmanError, Result},
    types::{FileMetadata, OperationResult, OperationType, ProgressInfo, Conflict, ConflictResolution},
    traits::{RestoreEngine, FileSystem, ProgressReporter},
};
use crate::config::Config;

/// Restore manager that orchestrates the restore process
pub struct RestoreManager<F, P> 
where 
    F: FileSystem + Send + Sync,
    P: ProgressReporter + Send + Sync,
{
    filesystem: F,
    progress_reporter: P,
    config: Config,
}

/// Restore session metadata
#[derive(Debug, Clone)]
pub struct RestoreSession {
    pub id: Uuid,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub backup_path: PathBuf,
    pub target_paths: Vec<PathBuf>,
    pub total_files: u64,
    pub processed_files: u64,
    pub conflicts: Vec<Conflict>,
    pub errors: Vec<String>,
}

impl<F, P> RestoreManager<F, P> 
where 
    F: FileSystem + Send + Sync,
    P: ProgressReporter + Send + Sync,
{
    /// Create a new restore manager
    pub fn new(filesystem: F, progress_reporter: P, config: Config) -> Self {
        Self {
            filesystem,
            progress_reporter,
            config,
        }
    }

    /// Start a new restore session
    #[instrument(skip(self))]
    pub async fn start_restore_session(&self, backup_path: PathBuf, target_paths: Vec<PathBuf>) -> Result<RestoreSession> {
        let session_id = Uuid::new_v4();

        info!(session_id = %session_id, "Starting restore session");

        // Verify backup exists
        if !self.filesystem.exists(&backup_path).await? {
            return Err(DotmanError::file_not_found(backup_path));
        }

        // Calculate total files for progress tracking
        let total_files = self.calculate_restore_size(&backup_path).await?;

        let session = RestoreSession {
            id: session_id,
            started_at: Utc::now(),
            backup_path,
            target_paths,
            total_files,
            processed_files: 0,
            conflicts: Vec::new(),
            errors: Vec::new(),
        };

        self.progress_reporter.report_progress(&ProgressInfo {
            current: 0,
            total: total_files,
            message: "Restore session started".to_string(),
            details: Some(format!("Session ID: {}", session_id)),
        });

        Ok(session)
    }

    /// Perform restore operation
    #[instrument(skip(self, session))]
    pub async fn restore(&self, mut session: RestoreSession) -> Result<RestoreSession> {
        info!(session_id = %session.id, "Starting restore operation");

        // Check for conflicts first
        self.detect_conflicts(&mut session).await?;

        // If there are conflicts and no automatic resolution strategy, return for user intervention
        if !session.conflicts.is_empty() {
            warn!(
                session_id = %session.id,
                conflicts = session.conflicts.len(),
                "Conflicts detected during restore"
            );
            return Ok(session);
        }

        // Perform the actual restore
        let target_paths = session.target_paths.clone();
        let backup_path = session.backup_path.clone();
        for target_path in &target_paths {
            match self.restore_path(&backup_path, &mut session).await {
                Ok(_) => {
                    debug!("Successfully restored: {}", target_path.display());
                }
                Err(e) => {
                    let error_msg = format!("Failed to restore {}: {}", target_path.display(), e);
                    error!("{}", error_msg);
                    session.errors.push(error_msg);
                }
            }
        }

        info!(
            session_id = %session.id,
            processed_files = session.processed_files,
            errors = session.errors.len(),
            "Restore session completed"
        );

        Ok(session)
    }

    /// Detect conflicts between backup and existing files
    async fn detect_conflicts(&self, session: &mut RestoreSession) -> Result<()> {
        info!(session_id = %session.id, "Detecting conflicts");

        // Walk through backup directory and check for conflicts
        let backup_path = session.backup_path.clone();
        self.walk_backup_directory(&backup_path, &backup_path, session).await?;

        Ok(())
    }

    /// Recursively walk backup directory to detect conflicts
    fn walk_backup_directory<'a>(
        &'a self,
        backup_root: &'a Path,
        current_path: &'a Path,
        session: &'a mut RestoreSession,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async move {
        let entries = self.filesystem.list_dir(current_path).await?;

        for entry in entries {
            let backup_file_path = current_path.join(&entry);
            let metadata = self.filesystem.metadata(&backup_file_path).await?;

            // Calculate target path
            let relative_path = backup_file_path.strip_prefix(backup_root)
                .map_err(|_| DotmanError::path("Invalid backup path structure".to_string()))?;
            let target_path = PathBuf::from("/").join(relative_path);

            match metadata.file_type {
                crate::core::types::FileType::File => {
                    if self.filesystem.exists(&target_path).await? {
                        let backup_metadata = self.filesystem.metadata(&backup_file_path).await?;
                        let current_metadata = self.filesystem.metadata(&target_path).await?;
                        session.conflicts.push(Conflict {
                            path: backup_file_path.clone(),
                            conflict_type: crate::core::types::ConflictType::PathOccupied,
                            backup_metadata,
                            current_metadata,
                            suggested_resolution: ConflictResolution::Ask,
                            resolution: Some(ConflictResolution::Ask),
                        });
                    }
                }
                crate::core::types::FileType::Directory => {
                    // Recursively check directory
                    self.walk_backup_directory(backup_root, &backup_file_path, session).await?;
                }
                crate::core::types::FileType::Symlink { .. } => {
                    if self.filesystem.exists(&target_path).await? {
                        let backup_metadata = self.filesystem.metadata(&backup_file_path).await?;
                        let current_metadata = self.filesystem.metadata(&target_path).await?;
                        session.conflicts.push(Conflict {
                            path: backup_file_path.clone(),
                            conflict_type: crate::core::types::ConflictType::SymlinkTargetMismatch,
                            backup_metadata,
                            current_metadata,
                            suggested_resolution: ConflictResolution::Ask,
                            resolution: Some(ConflictResolution::Ask),
                        });
                    }
                }
                _ => {
                    // Skip other file types
                }
            }
        }

        Ok(())
        })
    }

    /// Restore a single path (file or directory)
    #[instrument(skip(self, session))]
    async fn restore_path(
        &self, 
        backup_path: &Path, 
        session: &mut RestoreSession,
    ) -> Result<()> {
        let metadata = self.filesystem.metadata(backup_path).await?;

        // Calculate target path from backup path
        let relative_path = backup_path.strip_prefix(&session.backup_path)
            .map_err(|e| DotmanError::path(format!("Failed to strip prefix: {}", e)))?;
        let target_path = session.target_paths.first()
            .ok_or_else(|| DotmanError::restore("No target paths specified".to_string()))?
            .join(relative_path);

        match metadata.file_type {
            crate::core::types::FileType::File => {
                self.restore_file_impl(backup_path, &target_path, session).await
            }
            crate::core::types::FileType::Directory => {
                self.restore_directory_impl(backup_path, &target_path, session).await
            }
            crate::core::types::FileType::Symlink { .. } => {
                self.restore_symlink_impl(backup_path, &target_path, session).await
            }
            _ => {
                warn!("Unsupported file type for restore: {}", backup_path.display());
                Ok(())
            }
        }
    }

    /// Restore a single file
    async fn restore_file_impl(
        &self,
        backup_path: &Path,
        target_path: &Path,
        session: &mut RestoreSession,
    ) -> Result<()> {
        // Create parent directory if needed
        if let Some(parent) = target_path.parent() {
            self.filesystem.create_dir_all(parent).await?;
        }

        // Create backup of existing file if configured
        if self.config.create_backups && self.filesystem.exists(target_path).await? {
            let backup_file_path = target_path.with_extension("dotman.backup");
            self.filesystem.copy_file(target_path, &backup_file_path).await?;
            debug!("Created backup of existing file: {}", backup_file_path.display());
        }

        // Copy file from backup to target
        self.filesystem.copy_file(backup_path, target_path).await?;

        // Restore permissions if configured
        if self.config.preserve_permissions {
            let backup_metadata = self.filesystem.metadata(backup_path).await?;
            // TODO: Implement permission restoration
            debug!("Would restore permissions for: {}", target_path.display());
        }

        // Update progress
        session.processed_files += 1;

        self.progress_reporter.report_progress(&ProgressInfo {
            current: session.processed_files,
            total: session.total_files,
            message: format!("Restored: {}", target_path.display()),
            details: None,
        });

        Ok(())
    }

    /// Restore a directory recursively
    fn restore_directory_impl<'a>(
        &'a self,
        backup_path: &'a Path,
        target_path: &'a Path,
        session: &'a mut RestoreSession,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async move {
        // Create target directory
        self.filesystem.create_dir_all(target_path).await?;

        // Get directory entries
        let entries = self.filesystem.list_dir(backup_path).await?;

        for entry in entries {
            let backup_entry_path = backup_path.join(&entry);
            self.restore_path(&backup_entry_path, session).await?;
        }

        Ok(())
        })
    }

    /// Restore a symlink
    async fn restore_symlink_impl(
        &self,
        backup_path: &Path,
        target_path: &Path,
        session: &mut RestoreSession,
    ) -> Result<()> {
        // Create parent directory if needed
        if let Some(parent) = target_path.parent() {
            self.filesystem.create_dir_all(parent).await?;
        }

        // Read symlink target from backup
        let link_target = self.filesystem.read_symlink(backup_path).await?;

        // Remove existing file/symlink if it exists
        if self.filesystem.exists(target_path).await? {
            if self.config.create_backups {
                let backup_file_path = target_path.with_extension("dotman.backup");
                if matches!(self.filesystem.metadata(target_path).await?.file_type, crate::core::types::FileType::Symlink { .. }) {
                    // For symlinks, copy the link itself
                    let existing_target = self.filesystem.read_symlink(target_path).await?;
                    self.filesystem.create_symlink(&existing_target, &backup_file_path).await?;
                } else {
                    // For regular files, copy content
                    self.filesystem.copy_file(target_path, &backup_file_path).await?;
                }
            }
            self.filesystem.remove_file(target_path).await?;
        }

        // Create symlink
        self.filesystem.create_symlink(&link_target, target_path).await?;

        session.processed_files += 1;

        self.progress_reporter.report_progress(&ProgressInfo {
            current: session.processed_files,
            total: session.total_files,
            message: format!("Restored symlink: {}", target_path.display()),
            details: Some(format!("Target: {}", link_target.display())),
        });

        Ok(())
    }

    /// Calculate total file count for restore
    async fn calculate_restore_size(&self, backup_path: &Path) -> Result<u64> {
        self.count_files_in_backup(backup_path).await
    }

    /// Recursively count files in backup directory
    fn count_files_in_backup<'a>(&'a self, path: &'a Path) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<u64>> + Send + 'a>> {
        Box::pin(async move {
        let metadata = self.filesystem.metadata(path).await?;
        
        match metadata.file_type {
            crate::core::types::FileType::File => Ok(1),
            crate::core::types::FileType::Symlink { .. } => Ok(1),
            crate::core::types::FileType::Directory => {
                let mut total = 0u64;
                let entries = self.filesystem.list_dir(path).await?;
                for entry in entries {
                    let entry_path = path.join(&entry);
                    total += self.count_files_in_backup(&entry_path).await?;
                }
                Ok(total)
            }
            _ => Ok(0),
        }
        })
    }

    /// Resolve conflicts interactively or automatically
    pub async fn resolve_conflicts(&self, session: &mut RestoreSession, resolution_strategy: ConflictResolution) -> Result<()> {
        for conflict in &mut session.conflicts {
            conflict.resolution = Some(resolution_strategy.clone());
        }
        Ok(())
    }
}

#[async_trait]
impl<F, P> RestoreEngine for RestoreManager<F, P>
where 
    F: FileSystem + Send + Sync,
    P: ProgressReporter + Send + Sync,
{
    async fn restore_files(&self, backup_path: PathBuf, target_paths: Vec<PathBuf>) -> Result<Vec<OperationResult>> {
        let session = self.start_restore_session(backup_path, target_paths).await?;
        let completed_session = self.restore(session).await?;

        let mut results = Vec::new();
        
        for target_path in &completed_session.target_paths {
            let success = !completed_session.errors.iter()
                .any(|error| error.contains(&*target_path.to_string_lossy()));

            results.push(OperationResult {
                operation_type: OperationType::Restore,
                path: target_path.clone(),
                success,
                error: if success { 
                    None 
                } else { 
                    Some("Restore failed".to_string()) 
                },
                details: None,
                required_privileges: false,
                duration: None,
                bytes_processed: None,
            });
        }

        Ok(results)
    }

    async fn list_backup_contents(&self, backup_path: &Path) -> Result<Vec<FileMetadata>> {
        let mut contents = Vec::new();
        self.collect_backup_contents(backup_path, &mut contents).await?;
        Ok(contents)
    }

    async fn verify_restore(&self, target_paths: &[PathBuf]) -> Result<bool> {
        for path in target_paths {
            if !self.filesystem.exists(path).await? {
                return Ok(false);
            }
            // TODO: Add more comprehensive verification
            // - Check file integrity using hashes
            // - Verify metadata matches expectations
        }
        Ok(true)
    }
}

impl<F, P> RestoreManager<F, P>
where 
    F: FileSystem + Send + Sync,
    P: ProgressReporter + Send + Sync,
{
    /// Recursively collect backup contents
    fn collect_backup_contents<'a>(&'a self, path: &'a Path, contents: &'a mut Vec<FileMetadata>) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async move {
        let metadata = self.filesystem.metadata(path).await?;
        contents.push(metadata.clone());

        if metadata.file_type == crate::core::types::FileType::Directory {
            let entries = self.filesystem.list_dir(path).await?;
            for entry in entries {
                let entry_path = path.join(&entry);
                self.collect_backup_contents(&entry_path, contents).await?;
            }
        }

        Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filesystem::FileSystemImpl;
    use tempfile::TempDir;

    // Mock progress reporter for testing
    struct MockProgressReporter;

    impl ProgressReporter for MockProgressReporter {
        fn report_progress(&self, _progress: &ProgressInfo) {
            // Do nothing in tests
        }
    }

    #[tokio::test]
    async fn test_restore_manager_creation() {
        let filesystem = FileSystemImpl::new();
        let progress_reporter = MockProgressReporter;
        let config = Config::default();

        let manager = RestoreManager::new(filesystem, progress_reporter, config);
        
        // Just test that we can create the manager
        assert!(true);
    }

    #[tokio::test]
    async fn test_restore_session_creation() {
        let temp_dir = TempDir::new().unwrap();
        let filesystem = FileSystemImpl::new();
        let progress_reporter = MockProgressReporter;
        let config = Config::default();

        let manager = RestoreManager::new(filesystem, progress_reporter, config);

        // Create a backup directory with a test file
        let backup_dir = temp_dir.path().join("backup");
        tokio::fs::create_dir_all(&backup_dir).await.unwrap();
        tokio::fs::write(backup_dir.join("test.txt"), "test content").await.unwrap();

        let target_paths = vec![temp_dir.path().join("restored.txt")];
        
        let session = manager.start_restore_session(backup_dir, target_paths.clone()).await.unwrap();
        
        assert_eq!(session.target_paths, target_paths);
        assert!(session.total_files > 0);
    }
} 
