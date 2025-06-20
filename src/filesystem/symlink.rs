use std::path::{Path, PathBuf};
use async_trait::async_trait;
use std::os::unix::fs::symlink;

use crate::core::{
    error::{DotmanError, Result},
    types::{FileMetadata, FileType, OperationResult, OperationType},
    traits::FileHandler,
};

/// Specialized handler for symbolic links
pub struct SymlinkHandler;

impl SymlinkHandler {
    /// Create a new symlink handler
    pub fn new() -> Self {
        Self
    }

    /// Check if a symlink creates a loop
    pub async fn check_symlink_loop(&self, _link_path: &Path, target_path: &Path) -> Result<bool> {
        let mut visited = std::collections::HashSet::new();
        let mut current = target_path.to_path_buf();
        
        loop {
            if visited.contains(&current) {
                return Ok(true); // Loop detected
            }
            
            visited.insert(current.clone());
            
            match tokio::fs::symlink_metadata(&current).await {
                Ok(metadata) if metadata.file_type().is_symlink() => {
                    let new_target = tokio::fs::read_link(&current).await
                        .map_err(|e| DotmanError::symlink(format!("Failed to read symlink: {}", e)))?;
                    
                    // Handle relative paths
                    if new_target.is_relative() {
                        if let Some(parent) = current.parent() {
                            current = parent.join(new_target);
                        } else {
                            current = new_target;
                        }
                    } else {
                        current = new_target;
                    }
                }
                Ok(_) => break, // Not a symlink, end of chain
                Err(_) => break, // Target doesn't exist, but that's okay
            }
            
            // Prevent infinite loops by limiting iterations
            if visited.len() > 100 {
                return Ok(true);
            }
        }
        
        Ok(false)
    }

    /// Resolve symlink target to absolute path
    pub async fn resolve_target(&self, link_path: &Path, target: &Path) -> Result<PathBuf> {
        if target.is_absolute() {
            Ok(target.to_path_buf())
        } else {
            // Resolve relative to the directory containing the symlink
            if let Some(parent) = link_path.parent() {
                Ok(parent.join(target))
            } else {
                Ok(target.to_path_buf())
            }
        }
    }

    /// Check if symlink target exists
    pub async fn target_exists(&self, target: &Path) -> bool {
        tokio::fs::metadata(target).await.is_ok()
    }
}

#[async_trait]
impl FileHandler for SymlinkHandler {
    async fn get_metadata(&self, path: &Path) -> Result<FileMetadata> {
        let target = tokio::fs::read_link(path).await
            .map_err(|e| DotmanError::symlink(format!("Failed to read symlink: {}", e)))?;
        
        let is_absolute = target.is_absolute();
        let resolved_target = self.resolve_target(path, &target).await?;
        let target_exists = self.target_exists(&resolved_target).await;
        
        let file_type = FileType::Symlink {
            target: target.clone(),
            is_absolute,
            target_exists,
        };
        
        Ok(FileMetadata::new(path.to_path_buf(), file_type))
    }

    async fn copy(&self, src: &Path, dst: &Path, metadata: &FileMetadata) -> Result<OperationResult> {
        // For symlinks, recreate the symlink at the destination
        if let Some(target) = metadata.symlink_target() {
            symlink(target, dst)
                .map_err(|e| DotmanError::symlink(format!("Failed to create symlink: {}", e)))?;
            
            Ok(OperationResult {
                operation_type: OperationType::CreateSymlink,
                path: dst.to_path_buf(),
                success: true,
                error: None,
                details: Some(format!("Created symlink {} -> {}", dst.display(), target.display())),
                required_privileges: false,
                duration: None,
                bytes_processed: None,
            })
        } else {
            Err(DotmanError::symlink("Metadata does not contain symlink target"))
        }
    }

    async fn verify(&self, path: &Path, expected: &FileMetadata) -> Result<bool> {
        let actual = self.get_metadata(path).await?;
        
        if let (Some(actual_target), Some(expected_target)) = 
            (actual.symlink_target(), expected.symlink_target()) {
            Ok(actual_target == expected_target)
        } else {
            Ok(false)
        }
    }

    fn can_handle(&self, metadata: &FileMetadata) -> bool {
        metadata.is_symlink()
    }

    fn priority(&self) -> u32 {
        200 // Higher priority than regular files
    }
}

impl Default for SymlinkHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[cfg(unix)]
    #[tokio::test]
    async fn test_symlink_operations() {
        let temp_dir = TempDir::new().unwrap();
        let handler = SymlinkHandler::new();
        
        let target_file = temp_dir.path().join("target.txt");
        let symlink_file = temp_dir.path().join("link.txt");
        
        // Create target file
        tokio::fs::write(&target_file, b"test content").await.unwrap();
        
        // Create symlink
        tokio::fs::symlink(&target_file, &symlink_file).await.unwrap();
        
        // Test metadata
        let metadata = handler.get_metadata(&symlink_file).await.unwrap();
        assert!(metadata.is_symlink());
        
        if let Some(target) = metadata.symlink_target() {
            assert_eq!(target, &target_file);
        } else {
            panic!("Symlink should have target");
        }
        
        // Test loop detection
        let loop_link = temp_dir.path().join("loop.txt");
        tokio::fs::symlink(&loop_link, &loop_link).await.unwrap();
        
        let has_loop = handler.check_symlink_loop(&loop_link, &loop_link).await.unwrap();
        assert!(has_loop);
    }
} 