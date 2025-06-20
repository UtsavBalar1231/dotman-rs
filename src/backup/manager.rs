use std::path::{Path, PathBuf};
use async_trait::async_trait;
use uuid::Uuid;
use chrono::Utc;
use tracing::{info, warn, error, debug, instrument};
use serde::{Serialize, Deserialize};

use crate::core::{
    error::{DotmanError, Result},
    types::{OperationResult, OperationType, ProgressInfo},
    traits::{BackupEngine, FileSystem, ProgressReporter},
};
use crate::config::Config;

/// Backup manager that orchestrates the backup process
pub struct BackupManager<F, P> 
where 
    F: FileSystem + Send + Sync,
    P: ProgressReporter + Send + Sync,
{
    filesystem: F,
    progress_reporter: P,
    config: Config,
}

/// Backup session metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupSession {
    pub id: Uuid,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub source_paths: Vec<PathBuf>,
    pub backup_dir: PathBuf,
    pub total_files: u64,
    pub processed_files: u64,
    pub total_size: u64,
    pub processed_size: u64,
    pub errors: Vec<String>,
    /// Package name if this is a package backup
    pub package_name: Option<String>,
    /// Package description if this is a package backup
    pub package_description: Option<String>,
    /// Custom backup name/tag
    pub backup_name: Option<String>,
    /// Backup description
    pub backup_description: Option<String>,
}

impl<F, P> BackupManager<F, P> 
where 
    F: FileSystem + Send + Sync,
    P: ProgressReporter + Send + Sync,
{
    /// Create a new backup manager
    pub fn new(filesystem: F, progress_reporter: P, config: Config) -> Self {
        Self {
            filesystem,
            progress_reporter,
            config,
        }
    }

    /// Start a new backup session
    #[instrument(skip(self))]
    pub async fn start_backup_session(&self, source_paths: Vec<PathBuf>) -> Result<BackupSession> {
        self.start_backup_session_with_metadata(source_paths, None, None, None, None).await
    }

    /// Start a new backup session with package and naming metadata
    #[instrument(skip(self))]
    pub async fn start_backup_session_with_metadata(
        &self, 
        source_paths: Vec<PathBuf>,
        package_name: Option<String>,
        package_description: Option<String>,
        backup_name: Option<String>,
        backup_description: Option<String>,
    ) -> Result<BackupSession> {
        let session_id = Uuid::new_v4();
        
        // Create a more descriptive backup directory name
        let backup_dir_name = if let Some(ref pkg_name) = package_name {
            if let Some(ref name) = backup_name {
                format!("{}-{}", name, session_id)
            } else {
                format!("package-{}-{}", pkg_name, chrono::Utc::now().format("%Y%m%d-%H%M%S"))
            }
        } else if let Some(ref name) = backup_name {
            format!("{}-{}", name, session_id)
        } else {
            format!("backup-{}", session_id)
        };
        
        let backup_dir = self.config.backup_dir.join(backup_dir_name);

        info!(session_id = %session_id, "Starting backup session");

        // Create backup directory
        self.filesystem.create_dir_all(&backup_dir).await?;

        // Calculate total files and size for progress tracking
        let (total_files, total_size) = self.calculate_backup_size(&source_paths).await?;

        let session = BackupSession {
            id: session_id,
            started_at: Utc::now(),
            source_paths,
            backup_dir: backup_dir.clone(),
            total_files,
            processed_files: 0,
            total_size,
            processed_size: 0,
            errors: Vec::new(),
            package_name,
            package_description,
            backup_name,
            backup_description,
        };

        self.progress_reporter.report_progress(&ProgressInfo {
            current: 0,
            total: total_files,
            message: "Backup session started".to_string(),
            details: Some(format!("Session ID: {}", session_id)),
        });

        // Cleanup old backup versions if configured
        if self.config.max_backup_versions > 0 {
            self.cleanup_old_backups(&backup_dir).await?;
        }

        Ok(session)
    }

    /// Perform backup operation
    #[instrument(skip(self, session))]
    pub async fn backup(&self, mut session: BackupSession) -> Result<BackupSession> {
        info!(session_id = %session.id, "Starting backup operation");

        let source_paths = session.source_paths.clone();
        let backup_dir = session.backup_dir.clone();
        for source_path in &source_paths {
            match self.backup_path(source_path, &backup_dir, &mut session).await {
                Ok(_) => {
                    debug!("Successfully backed up: {}", source_path.display());
                }
                Err(e) => {
                    let error_msg = format!("Failed to backup {}: {}", source_path.display(), e);
                    error!("{}", error_msg);
                    session.errors.push(error_msg);
                }
            }
        }

        // Write session metadata
        self.write_session_metadata(&session).await?;

        info!(
            session_id = %session.id,
            processed_files = session.processed_files,
            errors = session.errors.len(),
            "Backup session completed"
        );

        Ok(session)
    }

    /// Backup a single path (file or directory)
    #[instrument(skip(self, session))]
    fn backup_path<'a>(
        &'a self, 
        source_path: &'a Path,
        backup_dir: &'a Path,
        session: &'a mut BackupSession,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async move {
        if !self.filesystem.exists(source_path).await? {
            return Err(DotmanError::file_not_found(source_path.to_path_buf()));
        }

        let metadata = self.filesystem.metadata(source_path).await?;
        
        // Check if path should be excluded
        if self.should_exclude_path(source_path)? {
            debug!("Excluding path: {}", source_path.display());
            return Ok(());
        }

        match metadata.file_type {
            crate::core::types::FileType::File => {
                self.backup_file(source_path, backup_dir, session).await
            }
            crate::core::types::FileType::Directory => {
                self.backup_directory(source_path, backup_dir, session).await
            }
            crate::core::types::FileType::Symlink { .. } => {
                self.backup_symlink(source_path, backup_dir, session).await
            }
            _ => {
                warn!("Unsupported file type for backup: {}", source_path.display());
                Ok(())
            }
        }
        })
    }

    /// Backup a single file
    async fn backup_file(
        &self,
        source_path: &Path,
        backup_dir: &Path,
        session: &mut BackupSession,
    ) -> Result<()> {
        let relative_path = self.get_relative_backup_path(source_path)?;
        let target_path = backup_dir.join(&relative_path);

        debug!("Backing up file: {} -> {}", source_path.display(), target_path.display());

        // Ensure parent directory exists
        if let Some(parent) = target_path.parent() {
            self.filesystem.create_dir_all(parent).await?;
        }

        // Copy file
        self.filesystem.copy_file(source_path, &target_path).await?;

        // Update progress
        session.processed_files += 1;
        if let Ok(metadata) = self.filesystem.metadata(source_path).await {
            session.processed_size += metadata.size;
        }

        self.progress_reporter.report_progress(&ProgressInfo {
            current: session.processed_files,
            total: session.total_files,
            message: format!("Backed up: {}", source_path.display()),
            details: None,
        });

        info!("Successfully backed up file: {} to {}", source_path.display(), target_path.display());
        Ok(())
    }

    /// Backup a directory and its contents
    async fn backup_directory(
        &self,
        source_path: &Path,
        backup_dir: &Path,
        session: &mut BackupSession,
    ) -> Result<()> {
        let relative_path = self.get_relative_backup_path(source_path)?;
        let target_dir = backup_dir.join(&relative_path);

        debug!("Backing up directory: {} -> {}", source_path.display(), target_dir.display());

        // Create the directory
        self.filesystem.create_dir_all(&target_dir).await?;

        // Recursively backup directory contents
        if let Ok(entries) = self.filesystem.list_dir(source_path).await {
            for entry in entries {
                let entry_path = source_path.join(&entry);
                self.backup_path(&entry_path, backup_dir, session).await?;
            }
        }

        info!("Successfully backed up directory: {}", source_path.display());
        Ok(())
    }

    /// Backup a symbolic link
    async fn backup_symlink(
        &self,
        source_path: &Path,
        backup_dir: &Path,
        session: &mut BackupSession,
    ) -> Result<()> {
        let relative_path = self.get_relative_backup_path(source_path)?;
        let target_path = backup_dir.join(&relative_path);

        debug!("Backing up symlink: {} -> {}", source_path.display(), target_path.display());

        // Read the symlink target
        let link_target = self.filesystem.read_symlink(source_path).await?;

        // Ensure parent directory exists
        if let Some(parent) = target_path.parent() {
            self.filesystem.create_dir_all(parent).await?;
        }

        // Create the symlink in backup
        self.filesystem.create_symlink(&link_target, &target_path).await?;

        // Update progress
        session.processed_files += 1;

        self.progress_reporter.report_progress(&ProgressInfo {
            current: session.processed_files,
            total: session.total_files,
            message: format!("Backed up symlink: {}", source_path.display()),
            details: None,
        });

        info!("Successfully backed up symlink: {} to {}", source_path.display(), target_path.display());
        Ok(())
    }

    /// Calculate total files and size for backup operation
    async fn calculate_backup_size(&self, source_paths: &[PathBuf]) -> Result<(u64, u64)> {
        let mut total_files = 0;
        let mut total_size = 0;

        for source_path in source_paths {
            if self.filesystem.exists(source_path).await? {
                let (files, size) = self.calculate_path_size(source_path).await?;
                total_files += files;
                total_size += size;
            }
        }

        Ok((total_files, total_size))
    }

    /// Calculate size and file count for a single path
    fn calculate_path_size<'a>(&'a self, path: &'a Path) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(u64, u64)>> + Send + 'a>> {
        Box::pin(async move {
        if !self.filesystem.exists(path).await? {
            return Ok((0, 0));
        }

        let metadata = self.filesystem.metadata(path).await?;

        match metadata.file_type {
            crate::core::types::FileType::File => {
                Ok((1, metadata.size))
            }
            crate::core::types::FileType::Directory => {
                let mut total_files = 0;
                let mut total_size = 0;

                if let Ok(entries) = self.filesystem.list_dir(path).await {
                    for entry in entries {
                        let entry_path = path.join(&entry);
                        let (files, size) = self.calculate_path_size(&entry_path).await?;
                        total_files += files;
                        total_size += size;
                    }
                }

                Ok((total_files, total_size))
            }
            crate::core::types::FileType::Symlink { .. } => {
                // Symlinks are counted as 1 file, but we could also read the target size
                Ok((1, metadata.size))
            }
            _ => Ok((0, 0)),
        }
        })
    }

    /// Check if a path should be excluded from backup
    fn should_exclude_path(&self, path: &Path) -> Result<bool> {
        let path_str = path.to_string_lossy();

        // Check exclude patterns
        for pattern in &self.config.exclude_patterns {
            if glob::Pattern::new(pattern)
                .map_err(|e| DotmanError::config(format!("Invalid exclude pattern '{}': {}", pattern, e)))?
                .matches(&path_str) {
                return Ok(true);
            }
        }

        // Check include patterns (if specified, only include matching paths)
        if !self.config.include_patterns.is_empty() {
            let mut included = false;
            for pattern in &self.config.include_patterns {
                if glob::Pattern::new(pattern)
                    .map_err(|e| DotmanError::config(format!("Invalid include pattern '{}': {}", pattern, e)))?
                    .matches(&path_str) {
                    included = true;
                    break;
                }
            }
            if !included {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Get relative path for backup storage
    fn get_relative_backup_path(&self, source_path: &Path) -> Result<PathBuf> {
        // For relative paths, use them as-is
        if source_path.is_relative() {
            return Ok(source_path.to_path_buf());
        }
        
        // For absolute paths, convert to relative path for backup storage
        // Remove leading slash to avoid issues with path joining
        let path_str = source_path.to_string_lossy();
        let relative_str = path_str.strip_prefix('/').unwrap_or(&path_str);
        Ok(PathBuf::from(relative_str))
    }

    /// Write session metadata to backup directory
    async fn write_session_metadata(&self, session: &BackupSession) -> Result<()> {
        let metadata_path = session.backup_dir.join("session_metadata.json");
        let metadata_json = serde_json::to_string_pretty(session)
            .map_err(|e| DotmanError::serialization(format!("Failed to serialize session metadata: {}", e)))?;

        self.filesystem.write_file(&metadata_path, metadata_json.as_bytes()).await
            .map_err(|e| DotmanError::filesystem(format!("Failed to write session metadata: {}", e)))
    }

    /// Clean up old backup directories
    async fn cleanup_old_backups(&self, backup_dir: &Path) -> Result<()> {
        if self.config.max_backup_versions == 0 {
            return Ok(()); // No cleanup needed
        }

        let max_versions = self.config.max_backup_versions as usize;
        let parent = backup_dir.parent().unwrap_or_else(|| Path::new("."));

        // Find all backup directories
        let mut backups = Vec::new();
        if let Ok(entries) = self.filesystem.list_dir(parent).await {
            for entry in entries {
                let entry_path = parent.join(&entry);
                let metadata = self.filesystem.metadata(&entry_path).await?;
                
                if metadata.is_directory() {
                    if let Some(name) = entry_path.file_name().and_then(|n| n.to_str()) {
                        if name.starts_with("backup-") {
                            backups.push((metadata.modified, entry_path));
                        }
                    }
                }
            }
        }

        // Sort by modification time (oldest first)
        backups.sort_by_key(|(modified, _)| *modified);

        // Remove excess backups
        if backups.len() > max_versions {
            let to_remove = backups.len() - max_versions;
            for (_, path) in backups.iter().take(to_remove) {
                info!("Removing old backup directory: {}", path.display());
                if let Err(e) = self.filesystem.remove(path).await {
                    warn!("Failed to remove old backup directory {}: {}", path.display(), e);
                }
            }
        }

        Ok(())
    }
}

#[async_trait]
impl<F, P> BackupEngine for BackupManager<F, P>
where 
    F: FileSystem + Send + Sync,
    P: ProgressReporter + Send + Sync,
{
    async fn backup_files(&self, source_paths: Vec<PathBuf>) -> Result<Vec<OperationResult>> {
        let session = self.start_backup_session(source_paths).await?;
        let completed_session = self.backup(session).await?;

        let mut results = Vec::new();
        
        for source_path in &completed_session.source_paths {
            let success = !completed_session.errors.iter()
                .any(|error| error.contains(&*source_path.to_string_lossy()));

            results.push(OperationResult {
                operation_type: OperationType::Backup,
                path: source_path.clone(),
                success,
                error: if success { 
                    None 
                } else { 
                    Some("Backup failed".to_string()) 
                },
                details: None,
                required_privileges: false,
                duration: None,
                bytes_processed: None,
            });
        }

        Ok(results)
    }

    async fn verify_backup(&self, backup_path: &Path) -> Result<bool> {
        // Basic verification - check if metadata file exists
        let metadata_path = backup_path.join("backup_metadata.json");
        
        if !self.filesystem.exists(&metadata_path).await? {
            return Ok(false);
        }

        // TODO: Add more comprehensive verification
        // - Check file integrity using hashes
        // - Verify all files are present
        // - Validate metadata consistency

        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filesystem::FileSystemImpl;
    use tempfile::TempDir;
    use std::sync::Arc;

    // Mock progress reporter for testing
    struct MockProgressReporter;

    impl ProgressReporter for MockProgressReporter {
        fn report_progress(&self, _progress: &ProgressInfo) {
            // Do nothing in tests
        }
    }

    #[tokio::test]
    async fn test_backup_manager_creation() {
        let filesystem = FileSystemImpl::new();
        let progress_reporter = MockProgressReporter;
        let config = Config::default();

        let manager = BackupManager::new(filesystem, progress_reporter, config);
        
        // Just test that we can create the manager
        assert!(true);
    }

    #[tokio::test]
    async fn test_backup_session_creation() {
        let temp_dir = TempDir::new().unwrap();
        let filesystem = FileSystemImpl::new();
        let progress_reporter = MockProgressReporter;
        let mut config = Config::default();
        config.backup_dir = temp_dir.path().to_path_buf();

        let manager = BackupManager::new(filesystem, progress_reporter, config);

        let source_paths = vec![temp_dir.path().join("test.txt")];
        
        // Create a test file
        tokio::fs::write(&source_paths[0], "test content").await.unwrap();

        let session = manager.start_backup_session(source_paths.clone()).await.unwrap();
        
        assert_eq!(session.source_paths, source_paths);
        assert!(session.backup_dir.exists());
    }
} 