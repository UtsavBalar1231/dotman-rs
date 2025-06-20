use std::path::Path;
use async_trait::async_trait;
use std::time::Instant;

use crate::core::{
    error::{DotmanError, Result},
    types::{FileMetadata, FileType, OperationResult, OperationType},
    traits::FileHandler,
};

/// Handler for regular files
pub struct RegularFileHandler;

#[async_trait]
impl FileHandler for RegularFileHandler {
    async fn get_metadata(&self, path: &Path) -> Result<FileMetadata> {
        let std_metadata = tokio::fs::metadata(path).await
            .map_err(|e| DotmanError::filesystem(format!("Failed to get metadata: {}", e)))?;
        
        // For now, return basic metadata - this will be enhanced later
        Ok(FileMetadata::new(path.to_path_buf(), FileType::File))
    }

    async fn copy(&self, src: &Path, dst: &Path, _metadata: &FileMetadata) -> Result<OperationResult> {
        let start = Instant::now();
        tokio::fs::copy(src, dst).await
            .map_err(|e| DotmanError::filesystem(format!("Failed to copy file: {}", e)))?;
        let size = tokio::fs::metadata(src).await?.len();
        let duration = start.elapsed();

        Ok(OperationResult {
            operation_type: OperationType::Copy,
            path: dst.to_path_buf(),
            success: true,
            error: None,
            details: Some(format!("Copied file to {}", dst.display())),
            required_privileges: false,
            duration: Some(duration),
            bytes_processed: Some(size),
        })
    }

    async fn verify(&self, path: &Path, expected: &FileMetadata) -> Result<bool> {
        let actual = self.get_metadata(path).await?;
        Ok(actual.size == expected.size && actual.permissions == expected.permissions)
    }

    fn can_handle(&self, metadata: &FileMetadata) -> bool {
        metadata.is_file()
    }

    fn priority(&self) -> u32 {
        100 // Standard priority
    }
}

/// Handler for directories
pub struct DirectoryHandler;

#[async_trait]
impl FileHandler for DirectoryHandler {
    async fn get_metadata(&self, path: &Path) -> Result<FileMetadata> {
        let _std_metadata = tokio::fs::metadata(path).await
            .map_err(|e| DotmanError::filesystem(format!("Failed to get metadata: {}", e)))?;
        
        Ok(FileMetadata::new(path.to_path_buf(), FileType::Directory))
    }

    async fn copy(&self, src: &Path, dst: &Path, _metadata: &FileMetadata) -> Result<OperationResult> {
        // Create destination directory
        tokio::fs::create_dir_all(dst).await
            .map_err(|e| DotmanError::filesystem(format!("Failed to create directory: {}", e)))?;

        Ok(OperationResult {
            operation_type: OperationType::Copy,
            path: dst.to_path_buf(),
            success: true,
            error: None,
            details: Some(format!("Created directory {}", dst.display())),
            required_privileges: false,
            duration: None,
            bytes_processed: None,
        })
    }

    async fn verify(&self, path: &Path, _expected: &FileMetadata) -> Result<bool> {
        Ok(tokio::fs::metadata(path).await?.is_dir())
    }

    fn can_handle(&self, metadata: &FileMetadata) -> bool {
        metadata.is_directory()
    }

    fn priority(&self) -> u32 {
        100
    }
} 