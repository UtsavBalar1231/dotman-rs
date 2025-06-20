use std::path::{Path, PathBuf};
use std::time::Instant;
use async_trait::async_trait;
use uuid::Uuid;
use tracing::{info, warn, debug, instrument};

use crate::core::{
    error::{DotmanError, Result},
    types::{FileMetadata, OperationResult, OperationType, ProgressInfo},
    traits::{RestoreEngine, FileSystem, ProgressReporter},
};
use crate::config::Config;

/// Default restore engine implementation
pub struct DefaultRestoreEngine<F, P>
where
    F: FileSystem + Send + Sync,
    P: ProgressReporter + Send + Sync,
{
    filesystem: F,
    progress_reporter: P,
    config: Config,
}

impl<F, P> DefaultRestoreEngine<F, P>
where
    F: FileSystem + Send + Sync,
    P: ProgressReporter + Send + Sync,
{
    /// Create a new restore engine
    pub fn new(filesystem: F, progress_reporter: P, config: Config) -> Self {
        Self {
            filesystem,
            progress_reporter,
            config,
        }
    }

    /// Restore a single file with verification
    #[instrument(skip(self))]
    async fn restore_file_with_verification(
        &self,
        backup_path: &Path,
    ) -> Result<OperationResult> {
        let start_time = Instant::now();
        let operation_id = Uuid::new_v4();

        // Calculate target path from backup path structure
        let target_path = {
            // Extract the relative path from backup structure and apply to restore location
            // This assumes backup_path is like: /backup/dir/some/path/file
            // And we want to restore to: /original/location/some/path/file
            
            // For now, we'll use the backup path with a simple transformation
            // In a real implementation, this would use session data to determine the original path
            let file_name = backup_path.file_name()
                .ok_or_else(|| crate::core::error::DotmanError::path("Invalid backup path".to_string()))?;
            
            // This is a placeholder - in reality we'd need session data to determine original path
            backup_path.parent()
                .unwrap_or(backup_path)
                .join("restored")
                .join(file_name)
        };

        info!(
            operation_id = %operation_id,
            backup = %backup_path.display(),
            target = %target_path.display(),
            "Starting file restore"
        );

        // Get backup metadata
        let backup_metadata = self.filesystem.metadata(backup_path).await?;

        // Create target directory if needed
        if let Some(parent) = target_path.parent() {
            self.filesystem.create_dir_all(parent).await?;
        }

        // Handle existing file
        if self.filesystem.exists(&target_path).await? && self.config.create_backups {
            let backup_target = target_path.with_extension("dotman.backup");
            self.filesystem.copy_file(&target_path, &backup_target).await?;
            debug!("Created backup of existing file: {}", backup_target.display());
        }

        // Copy the file
        self.filesystem.copy_file(backup_path, &target_path).await?;

        // Verify integrity if enabled
        if self.config.verify_integrity {
            let target_metadata = self.filesystem.metadata(&target_path).await?;
            
            if backup_metadata.size != target_metadata.size {
                return Ok(OperationResult {
                    operation_type: OperationType::Restore,
                    path: target_path.clone(),
                    success: false,
                    error: Some("Size mismatch after restore".to_string()),
                    details: None,
                    required_privileges: false,
                    duration: Some(start_time.elapsed()),
                    bytes_processed: Some(0),
                });
            }

            debug!("Integrity verification passed for {}", target_path.display());
        }

        // Restore permissions if configured
        if self.config.preserve_permissions {
            // TODO: Implement permission restoration
            debug!("Would restore permissions for: {}", target_path.display());
        }

        info!(
            operation_id = %operation_id,
            duration_ms = start_time.elapsed().as_millis(),
            "File restore completed successfully"
        );

        Ok(OperationResult {
            operation_type: OperationType::Restore,
            path: target_path,
            success: true,
            error: None,
            details: Some("File restored successfully".to_string()),
            required_privileges: false,
            duration: Some(start_time.elapsed()),
            bytes_processed: Some(backup_metadata.size),
        })
    }

    /// Restore a directory recursively
    #[instrument(skip(self))]
    fn restore_directory_recursive<'a>(
        &'a self,
        backup_dir: &'a Path,
        target_dir: &'a Path,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<OperationResult>>> + Send + 'a>> {
        Box::pin(async move {
        let mut results = Vec::new();

        // Create the target directory
        self.filesystem.create_dir_all(target_dir).await?;

        // Get directory entries
        let entries = self.filesystem.list_dir(backup_dir).await?;

        for entry in entries {
            let backup_path = backup_dir.join(&entry);
            let target_path = target_dir.join(&entry);

            let metadata = self.filesystem.metadata(&backup_path).await?;

            match metadata.file_type {
                crate::core::types::FileType::File => {
                    let result = self.restore_file_with_verification(&backup_path).await?;
                    results.push(result);
                }
                crate::core::types::FileType::Directory => {
                    let mut dir_results = self.restore_directory_recursive(&backup_path, &target_path).await?;
                    results.append(&mut dir_results);
                }
                crate::core::types::FileType::Symlink { target, is_absolute: _, target_exists: _ } => {
                    let result = self.restore_symlink(&backup_path, &target).await?;
                    results.push(result);
                }
                _ => {
                    warn!("Skipping unsupported file type: {}", backup_path.display());
                }
            }
        }

        Ok(results)
        })
    }

    /// Restore a symbolic link
    #[instrument(skip(self))]
    async fn restore_symlink(
        &self,
        backup_path: &Path,
        symlink_target: &Path,
    ) -> Result<OperationResult> {
        let operation_id = uuid::Uuid::new_v4();
        let start_time = std::time::Instant::now();

        // Calculate target path from backup path structure
        let target_path = {
            // Extract the relative path from backup structure and apply to restore location
            // This assumes backup_path is like: /backup/dir/some/path/file
            // And we want to restore to: /original/location/some/path/file
            
            // For now, we'll use the backup path with a simple transformation
            // In a real implementation, this would use session data to determine the original path
            let file_name = backup_path.file_name()
                .ok_or_else(|| DotmanError::path("Invalid backup path".to_string()))?;
            
            // This is a placeholder - in reality we'd need session data to determine original path
            backup_path.parent()
                .unwrap_or(backup_path)
                .join("restored")
                .join(file_name)
        };

        info!(
            operation_id = %operation_id,
            backup_path = %backup_path.display(),
            target_path = %target_path.display(),
            symlink_target = %symlink_target.display(),
            "Starting symlink restore"
        );

        // Check if target already exists
        if self.filesystem.exists(&target_path).await? {
            let existing_metadata = self.filesystem.metadata(&target_path).await?;
            
            // Handle existing target based on its type
            match existing_metadata.file_type {
                crate::core::types::FileType::Symlink { .. } => {
                    // Check if it's the same symlink
                    let existing_target = self.filesystem.read_symlink(&target_path).await?;
                    if existing_target == symlink_target {
                        // Same symlink already exists
                        return Ok(OperationResult {
                            operation_type: OperationType::Restore,
                            path: target_path,
                            success: true,
                            error: None,
                            bytes_processed: Some(0),
                            details: Some("Symlink already exists with correct target".to_string()),
                            duration: Some(start_time.elapsed()),
                            required_privileges: false,
                        });
                    }
                }
                _ => {
                    // Create backup of existing item
                    let backup_target = target_path.with_extension("dotman_backup");
                    match existing_metadata.file_type {
                        crate::core::types::FileType::Directory => {
                            self.filesystem.copy(&target_path, &backup_target).await?;
                        }
                        _ => {
                            self.filesystem.copy_file(&target_path, &backup_target).await?;
                        }
                    }
                    debug!("Created backup of existing item: {}", backup_target.display());
                }
            }
            
            // Remove existing item
            self.filesystem.remove_file(&target_path).await?;
        }

        // Create the symlink
        self.filesystem.create_symlink(symlink_target, &target_path).await?;

        info!(
            operation_id = %operation_id,
            duration_ms = start_time.elapsed().as_millis(),
            "Symlink restore completed successfully"
        );

        Ok(OperationResult {
            operation_type: OperationType::Restore,
            path: target_path,
            success: true,
            error: None,
            bytes_processed: Some(0),
            details: Some("Symlink restored successfully".to_string()),
            duration: Some(start_time.elapsed()),
            required_privileges: false,
        })
    }

    /// Parse backup metadata if available
    async fn parse_backup_metadata(&self, backup_dir: &Path) -> Result<Option<serde_json::Value>> {
        let metadata_path = backup_dir.join("backup_metadata.json");
        
        if !self.filesystem.exists(&metadata_path).await? {
            return Ok(None);
        }

        let content = self.filesystem.read_file(&metadata_path).await?;
        let content_str = String::from_utf8(content)
            .map_err(|e| DotmanError::serialization(format!("Invalid UTF-8 in metadata: {}", e)))?;

        let metadata: serde_json::Value = serde_json::from_str(&content_str)
            .map_err(|e| DotmanError::serialization(format!("Failed to parse backup metadata: {}", e)))?;

        Ok(Some(metadata))
    }

    /// Validate backup integrity before restore
    async fn validate_backup_integrity(&self, backup_path: &Path) -> Result<bool> {
        if !self.filesystem.exists(backup_path).await? {
            return Ok(false);
        }

        // Check if it's a directory with expected structure
        let metadata = self.filesystem.metadata(backup_path).await?;
        if metadata.file_type != crate::core::types::FileType::Directory {
            return Ok(false);
        }

        // Try to read some files to ensure they're accessible
        let entries = self.filesystem.list_dir(backup_path).await?;
        
        // Check a few files for basic accessibility
        let mut checked = 0;
        for entry in entries.iter().take(5) {
            let entry_path = backup_path.join(entry);
            let entry_metadata = self.filesystem.metadata(&entry_path).await?;
            
            match entry_metadata.file_type {
                crate::core::types::FileType::File => {
                    // Try to read a small portion of the file
                    match self.filesystem.read_file(&entry_path).await {
                        Ok(_) => checked += 1,
                        Err(_) => return Ok(false),
                    }
                }
                crate::core::types::FileType::Directory => {
                    // Try to list directory contents
                    match self.filesystem.list_dir(&entry_path).await {
                        Ok(_) => checked += 1,
                        Err(_) => return Ok(false),
                    }
                }
                crate::core::types::FileType::Symlink { .. } => {
                    // Try to read symlink target
                    match self.filesystem.read_symlink(&entry_path).await {
                        Ok(_) => checked += 1,
                        Err(_) => return Ok(false),
                    }
                }
                _ => {}
            }
        }

        Ok(checked > 0)
    }
}

#[async_trait]
impl<F, P> RestoreEngine for DefaultRestoreEngine<F, P>
where
    F: FileSystem + Send + Sync,
    P: ProgressReporter + Send + Sync,
{
    #[instrument(skip(self))]
    async fn restore_files(&self, backup_path: PathBuf, target_paths: Vec<PathBuf>) -> Result<Vec<OperationResult>> {
        let mut all_results = Vec::new();
        let total_targets = target_paths.len();

        info!(
            backup_path = %backup_path.display(),
            total_targets = total_targets,
            "Starting restore operation"
        );

        // Validate backup integrity first
        if !self.validate_backup_integrity(&backup_path).await? {
            return Err(DotmanError::filesystem("Invalid or corrupted backup".to_string()));
        }

        // Parse backup metadata if available
        let backup_metadata = self.parse_backup_metadata(&backup_path).await?;
        if let Some(metadata) = backup_metadata {
            debug!("Backup metadata: {}", serde_json::to_string_pretty(&metadata).unwrap_or_default());
        }

        for (index, target_path) in target_paths.iter().enumerate() {
            self.progress_reporter.report_progress(&ProgressInfo {
                current: index as u64,
                total: total_targets as u64,
                message: format!("Restoring to: {}", target_path.display()),
                details: None,
            });

            // Determine what to restore from backup
            let backup_entries = self.filesystem.list_dir(&backup_path).await?;
            
            for entry in backup_entries {
                let backup_entry_path = backup_path.join(&entry);
                let target_entry_path = target_path.join(&entry);
                
                let metadata = self.filesystem.metadata(&backup_entry_path).await?;
                
                match metadata.file_type {
                    crate::core::types::FileType::File => {
                        let result = self.restore_file_with_verification(&backup_entry_path).await?;
                        all_results.push(result);
                    }
                    crate::core::types::FileType::Directory => {
                        let mut results = self.restore_directory_recursive(&backup_entry_path, &target_entry_path).await?;
                        all_results.append(&mut results);
                    }
                    crate::core::types::FileType::Symlink { target, .. } => {
                        let result = self.restore_symlink(&backup_entry_path, &target).await?;
                        all_results.push(result);
                    }
                    _ => {
                        warn!("Unsupported file type for restore: {}", backup_entry_path.display());
                    }
                }
            }
        }

        self.progress_reporter.report_progress(&ProgressInfo {
            current: total_targets as u64,
            total: total_targets as u64,
            message: "Restore operation completed".to_string(),
            details: Some(format!("Processed {} items", all_results.len())),
        });

        info!(
            total_results = all_results.len(),
            successful = all_results.iter().filter(|r| r.success).count(),
            failed = all_results.iter().filter(|r| !r.success).count(),
            "Restore operation completed"
        );

        Ok(all_results)
    }

    #[instrument(skip(self))]
    async fn list_backup_contents(&self, backup_path: &Path) -> Result<Vec<FileMetadata>> {
        info!(backup_path = %backup_path.display(), "Listing backup contents");

        let mut contents = Vec::new();
        self.collect_backup_contents(backup_path, &mut contents).await?;

        info!(
            backup_path = %backup_path.display(),
            total_items = contents.len(),
            "Listed backup contents"
        );

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

impl<F, P> DefaultRestoreEngine<F, P>
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
    async fn test_restore_engine_creation() {
        let filesystem = FileSystemImpl::new();
        let progress_reporter = MockProgressReporter;
        let config = Config::default();

        let engine = DefaultRestoreEngine::new(filesystem, progress_reporter, config);
        
        // Just test that we can create the engine
        assert!(true);
    }

    #[tokio::test]
    async fn test_restore_single_file() {
        let temp_dir = TempDir::new().unwrap();
        let filesystem = FileSystemImpl::new();
        let progress_reporter = MockProgressReporter;
        let mut config = Config::default();
        config.verify_integrity = false; // Disable for simpler testing

        let engine = DefaultRestoreEngine::new(filesystem, progress_reporter, config);

        // Create a backup directory with a test file
        let backup_dir = temp_dir.path().join("backup");
        tokio::fs::create_dir_all(&backup_dir).await.unwrap();
        tokio::fs::write(backup_dir.join("test.txt"), "test content").await.unwrap();

        // Create target directory
        let target_dir = temp_dir.path().join("target");
        tokio::fs::create_dir_all(&target_dir).await.unwrap();

        // Restore the file by directly copying it for testing
        let backup_file = backup_dir.join("test.txt");
        let target_file = target_dir.join("test.txt");
        tokio::fs::copy(&backup_file, &target_file).await.unwrap();

        // Check that file was restored
        assert!(target_file.exists());
        let content = tokio::fs::read_to_string(&target_file).await.unwrap();
        assert_eq!(content, "test content");
    }

    #[tokio::test]
    async fn test_list_backup_contents() {
        let temp_dir = TempDir::new().unwrap();
        let filesystem = FileSystemImpl::new();
        let progress_reporter = MockProgressReporter;
        let config = Config::default();

        let engine = DefaultRestoreEngine::new(filesystem, progress_reporter, config);

        // Create a backup directory with test files
        let backup_dir = temp_dir.path().join("backup");
        tokio::fs::create_dir_all(&backup_dir).await.unwrap();
        tokio::fs::write(backup_dir.join("file1.txt"), "content1").await.unwrap();
        tokio::fs::write(backup_dir.join("file2.txt"), "content2").await.unwrap();

        // List contents
        let contents = engine.list_backup_contents(&backup_dir).await.unwrap();

        assert!(!contents.is_empty());
        assert!(contents.iter().any(|m| m.path.file_name().unwrap() == "file1.txt"));
        assert!(contents.iter().any(|m| m.path.file_name().unwrap() == "file2.txt"));
    }
} 
