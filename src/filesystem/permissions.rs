use std::path::Path;
use std::os::unix::fs::PermissionsExt;
use nix::unistd::{Uid, Gid, chown};

use crate::core::{
    error::{DotmanError, Result},
    types::FileMetadata,
};

/// Manager for file permissions and ownership
pub struct PermissionManager;

impl PermissionManager {
    /// Create a new permission manager
    pub fn new() -> Self {
        Self
    }

    /// Set file permissions
    pub async fn set_permissions(&self, path: &Path, mode: u32) -> Result<()> {
        let metadata = tokio::fs::metadata(path).await
            .map_err(|e| DotmanError::permission(format!("Failed to get metadata: {}", e)))?;
        
        let mut permissions = metadata.permissions();
        permissions.set_mode(mode);
        
        tokio::fs::set_permissions(path, permissions).await
            .map_err(|e| DotmanError::permission(format!("Failed to set permissions: {}", e)))
    }

    /// Set file ownership
    pub fn set_ownership(&self, path: &Path, uid: u32, gid: u32) -> Result<()> {
        let uid = Some(Uid::from_raw(uid));
        let gid = Some(Gid::from_raw(gid));
        
        chown(path, uid, gid)
            .map_err(|e| DotmanError::permission(format!("Failed to set ownership: {}", e)))
    }

    /// Get current user ID
    pub fn get_current_uid(&self) -> u32 {
        nix::unistd::getuid().as_raw()
    }

    /// Get current group ID
    pub fn get_current_gid(&self) -> u32 {
        nix::unistd::getgid().as_raw()
    }

    /// Check if running as root
    pub fn is_root(&self) -> bool {
        nix::unistd::getuid().is_root()
    }

    /// Apply metadata permissions to a file
    pub async fn apply_metadata(&self, path: &Path, metadata: &FileMetadata) -> Result<()> {
        // Set permissions
        self.set_permissions(path, metadata.permissions).await?;
        
        // Set ownership (only if we have privileges)
        if self.is_root() || metadata.uid == self.get_current_uid() {
            self.set_ownership(path, metadata.uid, metadata.gid)?;
        }
        
        Ok(())
    }
}

impl Default for PermissionManager {
    fn default() -> Self {
        Self::new()
    }
} 