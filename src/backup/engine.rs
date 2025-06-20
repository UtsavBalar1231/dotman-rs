use async_trait::async_trait;
use sha2::Digest;
use std::path::{Path, PathBuf};
use std::time::Instant;
use tracing::{debug, info, instrument, warn};
use uuid::Uuid;

use crate::config::Config;
use crate::core::{
    error::Result,
    traits::{BackupEngine, FileSystem, ProgressReporter},
    types::{OperationResult, OperationType, ProgressInfo},
};

/// Default backup engine implementation
pub struct DefaultBackupEngine<F, P>
where
    F: FileSystem + Send + Sync,
    P: ProgressReporter + Send + Sync,
{
    filesystem: F,
    progress_reporter: P,
    config: Config,
}

impl<F, P> DefaultBackupEngine<F, P>
where
    F: FileSystem + Send + Sync,
    P: ProgressReporter + Send + Sync,
{
    /// Create a new backup engine
    pub fn new(filesystem: F, progress_reporter: P, config: Config) -> Self {
        Self {
            filesystem,
            progress_reporter,
            config,
        }
    }

    /// Backup a single file with integrity verification
    #[instrument(skip(self))]
    async fn backup_file_with_verification(
        &self,
        source_path: &Path,
        backup_path: &Path,
    ) -> Result<OperationResult> {
        let start_time = Instant::now();
        let operation_id = Uuid::new_v4();

        info!(
            operation_id = %operation_id,
            source = %source_path.display(),
            backup = %backup_path.display(),
            "Starting file backup"
        );

        // Get source metadata
        let source_metadata = self.filesystem.metadata(source_path).await?;

        // Create backup directory if needed
        if let Some(parent) = backup_path.parent() {
            self.filesystem.create_dir_all(parent).await?;
        }

        // Copy the file
        self.filesystem.copy_file(source_path, backup_path).await?;

        // Verify integrity if enabled
        if self.config.verify_integrity {
            let backup_metadata = self.filesystem.metadata(backup_path).await?;

            if source_metadata.size != backup_metadata.size {
                return Ok(OperationResult {
                    operation_type: OperationType::Backup,
                    path: source_path.to_path_buf(),
                    success: false,
                    error: Some("Size mismatch after backup".to_string()),
                    details: None,
                    required_privileges: false,
                    duration: Some(start_time.elapsed()),
                    bytes_processed: Some(0),
                });
            }

            // TODO: Add hash verification
            debug!(
                "Integrity verification passed for {}",
                source_path.display()
            );
        }

        info!(
            operation_id = %operation_id,
            duration_ms = start_time.elapsed().as_millis(),
            "File backup completed successfully"
        );

        Ok(OperationResult {
            operation_type: OperationType::Backup,
            path: source_path.to_path_buf(),
            success: true,
            error: None,
            details: Some("File backed up successfully".to_string()),
            required_privileges: false,
            duration: Some(start_time.elapsed()),
            bytes_processed: Some(source_metadata.size),
        })
    }

    /// Backup a directory recursively
    #[instrument(skip(self))]
    fn backup_directory_recursive<'a>(
        &'a self,
        source_dir: &'a Path,
        backup_dir: &'a Path,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<OperationResult>>> + Send + 'a>> {
        Box::pin(async move {
        let start_time = Instant::now();
        let operation_id = Uuid::new_v4();
        let mut results = Vec::new();

        // Create the backup directory
        self.filesystem.create_dir_all(backup_dir).await?;

        // Get directory entries
        let entries = self.filesystem.list_dir(source_dir).await?;

        for entry in entries {
            let source_path = source_dir.join(&entry);
            let backup_path = backup_dir.join(&entry);

            let metadata = self.filesystem.metadata(&source_path).await?;

            match metadata.file_type {
                crate::core::types::FileType::File => {
                    let result = self
                        .backup_file_with_verification(&source_path, &backup_path)
                        .await?;
                    results.push(result);
                }
                crate::core::types::FileType::Directory => {
                    let mut dir_results = self
                        .backup_directory_recursive(&source_path, &backup_path)
                        .await?;
                    results.append(&mut dir_results);
                }
                crate::core::types::FileType::Symlink {  .. } => {
                    let result = self
                        .backup_symlink(&source_path, &backup_path)
                        .await?;
                    results.push(result);
                }
                _ => {
                    warn!("Skipping unsupported file type: {}", source_path.display());
                }
            }
        }

        info!(
            operation_id = %operation_id,
            duration_ms = start_time.elapsed().as_millis(),
            "Directory backup completed successfully"
        );

        Ok(results)
        })
    }

    /// Backup a symbolic link
    #[instrument(skip(self))]
    async fn backup_symlink(
        &self,
        source_path: &Path,
        backup_path: &Path,
    ) -> Result<OperationResult> {
        let start_time = Instant::now();
        let operation_id = Uuid::new_v4();

        // Read the symlink target from the source
        let target = self.filesystem.read_symlink(source_path).await?;

        info!(
            operation_id = %operation_id,
            source = %source_path.display(),
            backup = %backup_path.display(),
            target = %target.display(),
            "Backing up symlink"
        );

        // Create parent directory if needed
        if let Some(parent) = backup_path.parent() {
            self.filesystem.create_dir_all(parent).await?;
        }

        // Create the symlink in backup
        self.filesystem.create_symlink(&target, backup_path).await?;

        Ok(OperationResult {
            operation_type: OperationType::Backup,
            path: source_path.to_path_buf(),
            success: true,
            error: None,
            details: Some(format!("Symlink backed up: {} -> {}", source_path.display(), target.display())),
            required_privileges: false,
            duration: Some(start_time.elapsed()),
            bytes_processed: None,
        })
    }

    /// Clean up old backup versions
    async fn cleanup_old_versions(&self, backup_path: &Path) -> Result<()> {
        if self.config.max_backup_versions == 0 {
            return Ok(()); // No cleanup needed
        }

        let max_versions = self.config.max_backup_versions as usize;
        let parent = backup_path.parent().unwrap_or(backup_path);
        let stem = backup_path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("backup");

        // Find all versioned files
        let mut versions = Vec::new();
        if let Ok(entries) = self.filesystem.list_dir(parent).await {
            for entry in entries {
                let entry_path = parent.join(&entry);
                if let Some(extension) = entry_path.extension().and_then(|e| e.to_str()) {
                    if extension.starts_with('v') {
                        if let Ok(version_num) = extension[1..].parse::<u32>() {
                            versions.push((version_num, entry_path));
                        }
                    }
                }
            }
        }

        // Sort by version number
        versions.sort_by_key(|(version, _)| *version);

        // Remove excess versions
        if versions.len() > max_versions {
            let to_remove = versions.len() - max_versions;
            for (_, path) in versions.iter().take(to_remove) {
                debug!("Removing old backup version: {}", path.display());
                if let Err(e) = self.filesystem.remove_file(path).await {
                    warn!("Failed to remove old backup version {}: {}", path.display(), e);
                }
            }
        }

        Ok(())
    }

    /// Create versioned backup if needed
    async fn create_versioned_backup(&self, backup_path: &Path) -> Result<PathBuf> {
        if !self.filesystem.exists(backup_path).await? {
            return Ok(backup_path.to_path_buf());
        }

        // Cleanup old versions if max_versions is configured
        self.cleanup_old_versions(backup_path).await?;
        let max_versions = self.config.max_backup_versions;

        // Find next version number
        let mut version = 1;
        loop {
            let versioned_path = backup_path.with_extension(format!("v{}", version));
            if !self.filesystem.exists(&versioned_path).await? {
                return Ok(versioned_path);
            }
            version += 1;

            if version > max_versions {
                // Remove oldest version and use that slot
                let oldest_path = backup_path.with_extension("v1");
                self.filesystem.remove_file(&oldest_path).await?;

                // Shift all versions down
                for v in 2..=max_versions {
                    let from_path = backup_path.with_extension(format!("v{}", v));
                    let to_path = backup_path.with_extension(format!("v{}", v - 1));
                    if self.filesystem.exists(&from_path).await? {
                        self.filesystem.move_file(&from_path, &to_path).await?;
                    }
                }

                return Ok(backup_path.with_extension(format!("v{}", max_versions)));
            }
        }
    }
}

#[async_trait]
impl<F, P> BackupEngine for DefaultBackupEngine<F, P>
where
    F: FileSystem + Send + Sync,
    P: ProgressReporter + Send + Sync,
{
    #[instrument(skip(self))]
    async fn backup_files(&self, source_paths: Vec<PathBuf>) -> Result<Vec<OperationResult>> {
        let mut all_results = Vec::new();
        let total_paths = source_paths.len();

        info!(total_paths = total_paths, "Starting backup operation");

        for (index, source_path) in source_paths.iter().enumerate() {
            self.progress_reporter.report_progress(&ProgressInfo {
                current: index as u64,
                total: total_paths as u64,
                message: format!("Processing: {}", source_path.display()),
                details: None,
            });

            if !self.filesystem.exists(source_path).await? {
                warn!("Source path does not exist: {}", source_path.display());
                all_results.push(OperationResult {
                    path: source_path.clone(),
                    operation_type: OperationType::Backup,
                    success: false,
                    error: Some("Source path does not exist".to_string()),
                    details: None,
                    required_privileges: false,
                    duration: None,
                    bytes_processed: None,
                });
                continue;
            }

            let metadata = self.filesystem.metadata(source_path).await?;

            // Generate backup path
            let relative_path = source_path.strip_prefix("/").unwrap_or(source_path);
            let backup_path = self.config.backup_dir.join(relative_path);
            let versioned_backup_path = self.create_versioned_backup(&backup_path).await?;

            match metadata.file_type {
                crate::core::types::FileType::File => {
                    let result = self
                        .backup_file_with_verification(source_path, &versioned_backup_path)
                        .await?;
                    all_results.push(result);
                }
                crate::core::types::FileType::Directory => {
                    let mut results = self
                        .backup_directory_recursive(source_path, &versioned_backup_path)
                        .await?;
                    all_results.append(&mut results);
                }
                crate::core::types::FileType::Symlink {  .. } => {
                    let result = self
                        .backup_symlink(source_path, &versioned_backup_path)
                        .await?;
                    all_results.push(result);
                }
                _ => {
                    warn!(
                        "Unsupported file type for backup: {}",
                        source_path.display()
                    );
                }
            }
        }

        self.progress_reporter.report_progress(&ProgressInfo {
            current: total_paths as u64,
            total: total_paths as u64,
            message: "Backup operation completed".to_string(),
            details: Some(format!("Processed {} items", all_results.len())),
        });

        info!(
            total_results = all_results.len(),
            successful = all_results.iter().filter(|r| r.success).count(),
            failed = all_results.iter().filter(|r| !r.success).count(),
            "Backup operation completed"
        );

        Ok(all_results)
    }

    #[instrument(skip(self))]
    async fn verify_backup(&self, backup_path: &Path) -> Result<bool> {
        info!(backup_path = %backup_path.display(), "Verifying backup");

        if !self.filesystem.exists(backup_path).await? {
            warn!("Backup path does not exist: {}", backup_path.display());
            return Ok(false);
        }

        // TODO: Implement comprehensive backup verification
        // - Check metadata files
        // - Verify file integrity with hashes
        // - Ensure all expected files are present
        // - Validate directory structure

        // For now, just check if the path exists and is accessible
        let metadata = self.filesystem.metadata(backup_path).await?;

        match metadata.file_type {
            crate::core::types::FileType::Directory => {
                // For directories, check if we can list contents
                let _entries = self.filesystem.list_dir(backup_path).await?;
                Ok(true)
            }
            crate::core::types::FileType::File => {
                // For files, check if readable
                let _content = self.filesystem.read_file(backup_path).await?;
                Ok(true)
            }
            _ => {
                // Other types are considered valid if they exist
                Ok(true)
            }
        }
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
    async fn test_backup_engine_creation() {
        let filesystem = FileSystemImpl::new();
        let progress_reporter = MockProgressReporter;
        let config = Config::default();

        let engine = DefaultBackupEngine::new(filesystem, progress_reporter, config);

        // Just test that we can create the engine
        assert!(true);
    }

    #[tokio::test]
    async fn test_backup_single_file() {
        let temp_dir = TempDir::new().unwrap();
        let filesystem = FileSystemImpl::new();
        let progress_reporter = MockProgressReporter;
        let mut config = Config::default();
        config.backup_dir = temp_dir.path().join("backups");
        config.verify_integrity = false; // Disable for simpler testing

        let engine = DefaultBackupEngine::new(filesystem, progress_reporter, config);

        // Create a test file
        let test_file = temp_dir.path().join("test.txt");
        tokio::fs::write(&test_file, "test content").await.unwrap();

        // Backup the file
        let results = engine.backup_files(vec![test_file.clone()]).await.unwrap();

        assert_eq!(results.len(), 1);
        assert!(results[0].success);
        assert_eq!(results[0].path, test_file);
    }

    #[tokio::test]
    async fn test_backup_verification() {
        let temp_dir = TempDir::new().unwrap();
        let filesystem = FileSystemImpl::new();
        let progress_reporter = MockProgressReporter;
        let config = Config::default();

        let engine = DefaultBackupEngine::new(filesystem, progress_reporter, config);

        // Create a test file
        let test_file = temp_dir.path().join("test.txt");
        tokio::fs::write(&test_file, "test content").await.unwrap();

        // Test verification
        let is_valid = engine.verify_backup(&test_file).await.unwrap();
        assert!(is_valid);

        // Test non-existent file
        let non_existent = temp_dir.path().join("non_existent.txt");
        let is_valid = engine.verify_backup(&non_existent).await.unwrap();
        assert!(!is_valid);
    }
}

