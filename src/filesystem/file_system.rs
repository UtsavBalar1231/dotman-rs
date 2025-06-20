use std::path::{Path, PathBuf};
use std::time::SystemTime;
use std::collections::HashMap;
use async_trait::async_trait;
use tracing::{debug, warn};
use nix::sys::stat::SFlag;

use crate::core::{
    error::{DotmanError, Result},
    types::{FileMetadata, FileType},
    traits::FileSystem,
};

/// Real file system implementation
#[derive(Debug, Clone)]
pub struct FileSystemImpl {
    /// Whether to perform actual operations or just simulate them
    dry_run: bool,
}

impl FileSystemImpl {
    /// Create a new file system instance
    pub fn new() -> Self {
        Self { dry_run: false }
    }

    /// Create a new file system instance in dry-run mode
    pub fn new_dry_run() -> Self {
        Self { dry_run: true }
    }

    /// Convert system metadata to our FileMetadata type
    async fn metadata_from_std(&self, path: &Path, std_metadata: std::fs::Metadata) -> Result<FileMetadata> {
        use std::os::unix::fs::MetadataExt;
        
        let file_type = if std_metadata.file_type().is_file() {
            FileType::File
        } else if std_metadata.file_type().is_dir() {
            FileType::Directory
        } else if std_metadata.file_type().is_symlink() {
            let target = self.read_symlink(path).await?;
            let is_absolute = target.is_absolute();
            let target_exists = self.exists(&target).await?;
            
            FileType::Symlink {
                target,
                is_absolute,
                target_exists,
            }
        } else {
            // Try to determine special file types using nix
            match self.get_file_type_detailed(path).await {
                Ok(file_type) => file_type,
                Err(_) => FileType::Unknown,
            }
        };

        let modified = std_metadata
            .modified()
            .map_err(|e| DotmanError::filesystem(format!("Failed to get modified time: {}", e)))?
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_err(|e| DotmanError::filesystem(format!("Invalid modified time: {}", e)))?;

        let accessed = std_metadata
            .accessed()
            .map_err(|e| DotmanError::filesystem(format!("Failed to get accessed time: {}", e)))?
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_err(|e| DotmanError::filesystem(format!("Invalid accessed time: {}", e)))?;

        let created = std_metadata
            .created()
            .ok()
            .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok());

        // Check if path requires elevated privileges
        let requires_privileges = self.check_privileges_required(path).await;

        Ok(FileMetadata {
            path: path.to_path_buf(),
            file_type,
            size: std_metadata.len(),
            permissions: std_metadata.mode(),
            uid: std_metadata.uid(),
            gid: std_metadata.gid(),
            modified: chrono::DateTime::from_timestamp(modified.as_secs() as i64, modified.subsec_nanos())
                .unwrap_or_else(chrono::Utc::now),
            accessed: chrono::DateTime::from_timestamp(accessed.as_secs() as i64, accessed.subsec_nanos())
                .unwrap_or_else(chrono::Utc::now),
            created: created.and_then(|t| chrono::DateTime::from_timestamp(t.as_secs() as i64, t.subsec_nanos())),
            content_hash: None,
            directory_hash: None,
            extended_attributes: HashMap::new(), // TODO: Implement extended attributes
            requires_privileges,
        })
    }

    /// Get detailed file type using nix
    async fn get_file_type_detailed(&self, path: &Path) -> Result<FileType> {
        let stat = nix::sys::stat::lstat(path)
            .map_err(|e| DotmanError::filesystem(format!("Failed to stat file: {}", e)))?;

        let file_type = match SFlag::from_bits_truncate(stat.st_mode) {
            s if s.contains(SFlag::S_IFREG) => FileType::File,
            s if s.contains(SFlag::S_IFDIR) => FileType::Directory,
            s if s.contains(SFlag::S_IFLNK) => {
                let target = self.read_symlink(path).await?;
                let is_absolute = target.is_absolute();
                let target_exists = self.exists(&target).await?;
                
                FileType::Symlink {
                    target,
                    is_absolute,
                    target_exists,
                }
            }
            s if s.contains(SFlag::S_IFCHR) => FileType::CharDevice,
            s if s.contains(SFlag::S_IFBLK) => FileType::BlockDevice,
            s if s.contains(SFlag::S_IFIFO) => FileType::Fifo,
            s if s.contains(SFlag::S_IFSOCK) => FileType::Socket,
            _ => FileType::Unknown,
        };

        Ok(file_type)
    }

    /// Check if a path requires elevated privileges to access
    async fn check_privileges_required(&self, path: &Path) -> bool {
        // Check if path is in system directories
        let system_paths = [
            "/etc", "/usr", "/var", "/boot", "/opt", "/srv", "/root"
        ];
        
        if let Some(path_str) = path.to_str() {
            if system_paths.iter().any(|&sys_path| path_str.starts_with(sys_path)) {
                return true;
            }
        }

        // Check if we can access the file without elevated privileges
        match tokio::fs::metadata(path).await {
            Ok(_) => false,
            Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => true,
            Err(_) => false,
        }
    }

    /// Copy file contents with proper error handling
    async fn copy_file_contents(&self, src: &Path, dst: &Path) -> Result<()> {
        if self.dry_run {
            debug!("DRY RUN: Would copy file {} to {}", src.display(), dst.display());
            return Ok(());
        }

        debug!("Copying file {} to {}", src.display(), dst.display());

        // Ensure destination directory exists
        if let Some(parent) = dst.parent() {
            self.create_dir_all(parent).await?;
        }

        // Copy the file
        tokio::fs::copy(src, dst).await
            .map_err(|e| DotmanError::filesystem(format!("Failed to copy file: {}", e)))?;

        Ok(())
    }

    /// Copy directory recursively
    async fn copy_dir_recursive(&self, src: &Path, dst: &Path) -> Result<()> {
        if self.dry_run {
            debug!("DRY RUN: Would copy directory {} to {}", src.display(), dst.display());
            return Ok(());
        }

        debug!("Copying directory {} to {}", src.display(), dst.display());

        self.create_dir_all(dst).await?;

        let mut entries = tokio::fs::read_dir(src).await
            .map_err(|e| DotmanError::filesystem(format!("Failed to read directory: {}", e)))?;

        while let Some(entry) = entries.next_entry().await
            .map_err(|e| DotmanError::filesystem(format!("Failed to read directory entry: {}", e)))? {
            
            let src_path = entry.path();
            let dst_path = dst.join(entry.file_name());
            
            let metadata = entry.metadata().await
                .map_err(|e| DotmanError::filesystem(format!("Failed to get entry metadata: {}", e)))?;

            if metadata.is_dir() {
                Box::pin(self.copy_dir_recursive(&src_path, &dst_path)).await?;
            } else if metadata.is_file() {
                self.copy_file_contents(&src_path, &dst_path).await?;
            } else if metadata.file_type().is_symlink() {
                let target = self.read_symlink(&src_path).await?;
                self.create_symlink(&target, &dst_path).await?;
            }
        }

        Ok(())
    }
}

impl Default for FileSystemImpl {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl FileSystem for FileSystemImpl {
    async fn exists(&self, path: &Path) -> Result<bool> {
        Ok(tokio::fs::metadata(path).await.is_ok())
    }

    async fn create_dir_all(&self, path: &Path) -> Result<()> {
        if self.dry_run {
            debug!("DRY RUN: Would create directory {}", path.display());
            return Ok(());
        }

        debug!("Creating directory {}", path.display());
        
        tokio::fs::create_dir_all(path).await
            .map_err(|e| DotmanError::filesystem(format!("Failed to create directory: {}", e)))
    }

    async fn remove(&self, path: &Path) -> Result<()> {
        if self.dry_run {
            debug!("DRY RUN: Would remove {}", path.display());
            return Ok(());
        }

        debug!("Removing {}", path.display());

        let metadata = tokio::fs::metadata(path).await
            .map_err(|e| DotmanError::filesystem(format!("Failed to get metadata for removal: {}", e)))?;

        if metadata.is_dir() {
            tokio::fs::remove_dir_all(path).await
                .map_err(|e| DotmanError::filesystem(format!("Failed to remove directory: {}", e)))
        } else {
            tokio::fs::remove_file(path).await
                .map_err(|e| DotmanError::filesystem(format!("Failed to remove file: {}", e)))
        }
    }

    async fn copy(&self, src: &Path, dst: &Path) -> Result<()> {
        let src_metadata = tokio::fs::metadata(src).await
            .map_err(|e| DotmanError::filesystem(format!("Failed to get source metadata: {}", e)))?;

        if src_metadata.is_dir() {
            self.copy_dir_recursive(src, dst).await
        } else {
            self.copy_file_contents(src, dst).await
        }
    }

    async fn move_file(&self, src: &Path, dst: &Path) -> Result<()> {
        if self.dry_run {
            debug!("DRY RUN: Would move {} to {}", src.display(), dst.display());
            return Ok(());
        }

        debug!("Moving {} to {}", src.display(), dst.display());

        // Ensure destination directory exists
        if let Some(parent) = dst.parent() {
            self.create_dir_all(parent).await?;
        }

        tokio::fs::rename(src, dst).await
            .map_err(|e| DotmanError::filesystem(format!("Failed to move file: {}", e)))
    }

    async fn metadata(&self, path: &Path) -> Result<FileMetadata> {
        // Use symlink_metadata to properly handle symlinks
        let std_metadata = tokio::fs::symlink_metadata(path).await
            .map_err(|e| DotmanError::filesystem(format!("Failed to get metadata: {}", e)))?;

        self.metadata_from_std(path, std_metadata).await
    }

    async fn list_dir(&self, path: &Path) -> Result<Vec<FileMetadata>> {
        let mut entries = tokio::fs::read_dir(path).await
            .map_err(|e| DotmanError::filesystem(format!("Failed to read directory: {}", e)))?;

        let mut result = Vec::new();

        while let Some(entry) = entries.next_entry().await
            .map_err(|e| DotmanError::filesystem(format!("Failed to read directory entry: {}", e)))? {
            
            let entry_path = entry.path();
            match self.metadata(&entry_path).await {
                Ok(metadata) => result.push(metadata),
                Err(e) => {
                    warn!("Failed to get metadata for {}: {}", entry_path.display(), e);
                    continue;
                }
            }
        }

        Ok(result)  
    }

    async fn read_file(&self, path: &Path) -> Result<Vec<u8>> {
        debug!("Reading file {}", path.display());
        
        tokio::fs::read(path).await
            .map_err(|e| DotmanError::filesystem(format!("Failed to read file: {}", e)))
    }

    async fn write_file(&self, path: &Path, contents: &[u8]) -> Result<()> {
        if self.dry_run {
            debug!("DRY RUN: Would write {} bytes to {}", contents.len(), path.display());
            return Ok(());
        }

        debug!("Writing {} bytes to {}", contents.len(), path.display());

        // Ensure directory exists
        if let Some(parent) = path.parent() {
            self.create_dir_all(parent).await?;
        }

        tokio::fs::write(path, contents).await
            .map_err(|e| DotmanError::filesystem(format!("Failed to write file: {}", e)))
    }

    async fn create_symlink(&self, target: &Path, link: &Path) -> Result<()> {
        if self.dry_run {
            debug!("DRY RUN: Would create symlink {} -> {}", link.display(), target.display());
            return Ok(());
        }

        debug!("Creating symlink {} -> {}", link.display(), target.display());

        // Ensure directory exists
        if let Some(parent) = link.parent() {
            self.create_dir_all(parent).await?;
        }

        #[cfg(unix)]
        {
            tokio::fs::symlink(target, link).await
                .map_err(|e| DotmanError::filesystem(format!("Failed to create symlink: {}", e)))
        }

        #[cfg(not(unix))]
        {
            Err(DotmanError::filesystem("Symlinks not supported on this platform".to_string()))
        }
    }

    async fn read_symlink(&self, path: &Path) -> Result<PathBuf> {
        debug!("Reading symlink {}", path.display());
        
        tokio::fs::read_link(path).await
            .map_err(|e| DotmanError::filesystem(format!("Failed to read symlink: {}", e)))
    }

    async fn copy_file(&self, src: &Path, dst: &Path) -> Result<()> {
        self.copy_file_contents(src, dst).await
    }

    async fn remove_file(&self, path: &Path) -> Result<()> {
        if self.dry_run {
            debug!("DRY RUN: Would remove file {}", path.display());
            return Ok(());
        }

        debug!("Removing file {}", path.display());
        tokio::fs::remove_file(path).await
            .map_err(|e| DotmanError::filesystem(format!("Failed to remove file: {}", e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_file_system_basic_operations() {
        let temp_dir = TempDir::new().unwrap();
        let fs = FileSystemImpl::new();

        let test_file = temp_dir.path().join("test.txt");
        let test_content = b"Hello, World!";

        // Test write
        fs.write_file(&test_file, test_content).await.unwrap();

        // Test exists
        assert!(fs.exists(&test_file).await.unwrap());

        // Test read
        let content = fs.read_file(&test_file).await.unwrap();
        assert_eq!(content, test_content);

        // Test metadata
        let metadata = fs.metadata(&test_file).await.unwrap();
        assert!(metadata.is_file());
        assert_eq!(metadata.size, test_content.len() as u64);

        // Test remove
        fs.remove(&test_file).await.unwrap();
        assert!(!fs.exists(&test_file).await.unwrap());
    }

    #[tokio::test]
    async fn test_directory_operations() {
        let temp_dir = TempDir::new().unwrap();
        let fs = FileSystemImpl::new();

        let test_dir = temp_dir.path().join("subdir");
        let test_file = test_dir.join("file.txt");

        // Test create directory
        fs.create_dir_all(&test_dir).await.unwrap();
        assert!(fs.exists(&test_dir).await.unwrap());

        // Test write file in directory
        fs.write_file(&test_file, b"test").await.unwrap();

        // Test list directory
        let entries = fs.list_dir(&test_dir).await.unwrap();
        assert_eq!(entries.len(), 1);
        assert!(entries[0].is_file());

        // Test copy directory
        let copy_dir = temp_dir.path().join("copy");
        fs.copy(&test_dir, &copy_dir).await.unwrap();
        
        let copy_file = copy_dir.join("file.txt");
        assert!(fs.exists(&copy_file).await.unwrap());
        
        let content = fs.read_file(&copy_file).await.unwrap();
        assert_eq!(content, b"test");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn test_symlink_operations() {
        let temp_dir = TempDir::new().unwrap();
        let fs = FileSystemImpl::new();

        let target_file = temp_dir.path().join("target.txt");
        let symlink_file = temp_dir.path().join("link.txt");

        // Create target file
        fs.write_file(&target_file, b"target content").await.unwrap();

        // Create symlink
        fs.create_symlink(&target_file, &symlink_file).await.unwrap();
        assert!(fs.exists(&symlink_file).await.unwrap());

        // Test read symlink
        let link_target = fs.read_symlink(&symlink_file).await.unwrap();
        assert_eq!(link_target, target_file);

        // Test metadata for symlink
        let metadata = fs.metadata(&symlink_file).await.unwrap();
        assert!(metadata.is_symlink());
        
        if let Some(target) = metadata.symlink_target() {
            assert_eq!(target, &target_file);
        } else {
            panic!("Symlink should have target");
        }
    }

    #[tokio::test]
    async fn test_dry_run_mode() {
        let temp_dir = TempDir::new().unwrap();
        let fs = FileSystemImpl::new_dry_run();

        let test_file = temp_dir.path().join("test.txt");

        // In dry run mode, operations should not actually happen
        fs.write_file(&test_file, b"test").await.unwrap();
        assert!(!fs.exists(&test_file).await.unwrap());

        fs.create_dir_all(&test_file.parent().unwrap()).await.unwrap();
        // Directory creation is also dry run, so it shouldn't exist
        // But we can't test this easily without more complex setup
    }
} 