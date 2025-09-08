use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

/// Cross-platform file permissions handling
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FilePermissions {
    mode: u32,
}

impl FilePermissions {
    /// Create permissions from a raw mode value
    #[must_use]
    pub const fn from_mode(mode: u32) -> Self {
        Self { mode }
    }

    /// Get the raw mode value
    #[must_use]
    pub const fn mode(&self) -> u32 {
        self.mode
    }

    /// Read permissions from a file
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Failed to read file metadata
    pub fn from_path(path: &Path) -> Result<Self> {
        let metadata = fs::metadata(path)
            .with_context(|| format!("Failed to read metadata for: {}", path.display()))?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            Ok(Self::from_mode(metadata.mode()))
        }

        #[cfg(windows)]
        {
            // On Windows, we'll store a simplified permission model
            // Read-only flag is the main permission we can preserve
            let mode = if metadata.permissions().readonly() {
                0o444 // Read-only for all
            } else {
                0o644 // Read-write for owner, read for others
            };
            Ok(Self::from_mode(mode))
        }

        #[cfg(not(any(unix, windows)))]
        {
            // For other platforms, use a default permission
            Ok(Self::from_mode(0o644))
        }
    }

    /// Apply permissions to a file if `preserve_permissions` is enabled
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Failed to set file permissions (only on platforms that support it)
    pub fn apply_to_path(&self, path: &Path, preserve_permissions: bool) -> Result<()> {
        if !preserve_permissions {
            return Ok(());
        }

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let permissions = fs::Permissions::from_mode(self.mode);
            fs::set_permissions(path, permissions)
                .with_context(|| format!("Failed to set permissions for: {}", path.display()))?;
        }

        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;

            // On Windows, we can only reliably set the read-only flag
            // Check if the stored mode indicates read-only (no write bits)
            let is_readonly = (self.mode & 0o200) == 0; // Check owner write bit

            let metadata = fs::metadata(path)
                .with_context(|| format!("Failed to read metadata for: {}", path.display()))?;

            let mut permissions = metadata.permissions();
            permissions.set_readonly(is_readonly);

            fs::set_permissions(path, permissions)
                .with_context(|| format!("Failed to set permissions for: {}", path.display()))?;
        }

        #[cfg(not(any(unix, windows)))]
        {
            // On other platforms, silently skip permission setting
            // This ensures the code compiles and runs but doesn't fail
            let _ = path; // Suppress unused variable warning
        }

        Ok(())
    }

    /// Get a platform-appropriate default permission mode
    #[must_use]
    pub const fn default_file() -> Self {
        #[cfg(unix)]
        {
            Self::from_mode(0o644) // rw-r--r--
        }

        #[cfg(not(unix))]
        {
            Self::from_mode(0o644) // Use same representation for consistency
        }
    }

    /// Get a platform-appropriate default directory permission mode
    #[must_use]
    pub const fn default_directory() -> Self {
        #[cfg(unix)]
        {
            Self::from_mode(0o755) // rwxr-xr-x
        }

        #[cfg(not(unix))]
        {
            Self::from_mode(0o755) // Use same representation for consistency
        }
    }

    /// Check if the permissions indicate an executable file
    #[must_use]
    pub const fn is_executable(&self) -> bool {
        #[cfg(unix)]
        {
            // Check if any execute bit is set
            (self.mode & 0o111) != 0
        }

        #[cfg(not(unix))]
        {
            // On non-Unix platforms, we can't reliably determine executability
            // from permissions alone
            false
        }
    }

    /// Create executable permissions
    #[must_use]
    pub const fn executable() -> Self {
        #[cfg(unix)]
        {
            Self::from_mode(0o755) // rwxr-xr-x
        }

        #[cfg(not(unix))]
        {
            Self::from_mode(0o755) // Use same representation
        }
    }
}

impl Default for FilePermissions {
    fn default() -> Self {
        Self::default_file()
    }
}

/// Helper function to preserve permissions when copying a file
///
/// # Errors
///
/// Returns an error if:
/// - Failed to copy the file
/// - Failed to preserve permissions (if enabled and supported)
pub fn copy_with_permissions(src: &Path, dst: &Path, preserve_permissions: bool) -> Result<()> {
    // Copy the file content
    fs::copy(src, dst)
        .with_context(|| format!("Failed to copy {} to {}", src.display(), dst.display()))?;

    // Preserve permissions if requested
    if preserve_permissions {
        let permissions = FilePermissions::from_path(src)?;
        permissions.apply_to_path(dst, true)?;
    }

    Ok(())
}

/// Helper to check if the platform supports full permission preservation
#[must_use]
pub const fn supports_full_permissions() -> bool {
    cfg!(unix)
}

/// Helper to check if the platform supports any permission preservation
#[must_use]
pub const fn supports_any_permissions() -> bool {
    cfg!(any(unix, windows))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_default_permissions() {
        let file_perms = FilePermissions::default_file();
        assert_eq!(file_perms.mode(), 0o644);

        let dir_perms = FilePermissions::default_directory();
        assert_eq!(dir_perms.mode(), 0o755);
    }

    #[test]
    fn test_from_mode() {
        let perms = FilePermissions::from_mode(0o755);
        assert_eq!(perms.mode(), 0o755);
    }

    #[test]
    fn test_is_executable() {
        let exec_perms = FilePermissions::from_mode(0o755);
        let non_exec_perms = FilePermissions::from_mode(0o644);

        #[cfg(unix)]
        {
            assert!(exec_perms.is_executable());
            assert!(!non_exec_perms.is_executable());
        }

        #[cfg(not(unix))]
        {
            // On non-Unix, is_executable always returns false
            assert!(!exec_perms.is_executable());
            assert!(!non_exec_perms.is_executable());
        }
    }

    #[test]
    fn test_read_and_apply_permissions() -> Result<()> {
        let dir = tempdir()?;
        let file_path = dir.path().join("test.txt");

        // Create a test file
        fs::write(&file_path, "test content")?;

        // Read permissions
        let perms = FilePermissions::from_path(&file_path)?;

        // Create another file and apply the same permissions
        let file2_path = dir.path().join("test2.txt");
        fs::write(&file2_path, "test content 2")?;

        // Apply permissions (with preserve_permissions = true)
        perms.apply_to_path(&file2_path, true)?;

        // Verify permissions were applied (platform-dependent)
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            let meta1 = fs::metadata(&file_path)?;
            let meta2 = fs::metadata(&file2_path)?;
            // Check that permission bits match (mask out file type bits)
            assert_eq!(meta1.mode() & 0o777, meta2.mode() & 0o777);
        }

        Ok(())
    }

    #[test]
    fn test_apply_permissions_disabled() -> Result<()> {
        let dir = tempdir()?;
        let file_path = dir.path().join("test.txt");
        fs::write(&file_path, "test")?;

        let perms = FilePermissions::from_mode(0o755);

        // Apply with preserve_permissions = false should not error
        perms.apply_to_path(&file_path, false)?;

        Ok(())
    }

    #[test]
    fn test_copy_with_permissions() -> Result<()> {
        let dir = tempdir()?;
        let src = dir.path().join("source.txt");
        let dst = dir.path().join("dest.txt");

        fs::write(&src, "content")?;

        // Set specific permissions on source
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = fs::Permissions::from_mode(0o600);
            fs::set_permissions(&src, perms)?;
        }

        // Copy with permissions
        copy_with_permissions(&src, &dst, true)?;

        // Verify file was copied
        assert!(dst.exists());
        let content = fs::read_to_string(&dst)?;
        assert_eq!(content, "content");

        // Verify permissions were preserved (platform-dependent)
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            let src_meta = fs::metadata(&src)?;
            let dst_meta = fs::metadata(&dst)?;
            assert_eq!(src_meta.mode() & 0o777, dst_meta.mode() & 0o777);
        }

        Ok(())
    }

    #[test]
    fn test_copy_without_permissions() -> Result<()> {
        let dir = tempdir()?;
        let src = dir.path().join("source.txt");
        let dst = dir.path().join("dest.txt");

        fs::write(&src, "content")?;

        // Copy without preserving permissions
        copy_with_permissions(&src, &dst, false)?;

        // Verify file was copied
        assert!(dst.exists());
        let content = fs::read_to_string(&dst)?;
        assert_eq!(content, "content");

        Ok(())
    }

    #[test]
    #[cfg(windows)]
    fn test_windows_readonly_handling() -> Result<()> {
        let dir = tempdir()?;
        let file_path = dir.path().join("readonly.txt");

        // Create a file and make it read-only
        fs::write(&file_path, "test")?;
        let metadata = fs::metadata(&file_path)?;
        let mut perms = metadata.permissions();
        perms.set_readonly(true);
        fs::set_permissions(&file_path, perms)?;

        // Read permissions - should detect read-only
        let file_perms = FilePermissions::from_path(&file_path)?;
        assert_eq!(file_perms.mode(), 0o444);

        // Apply to another file
        let file2_path = dir.path().join("readonly2.txt");
        fs::write(&file2_path, "test2")?;
        file_perms.apply_to_path(&file2_path, true)?;

        // Verify the second file is also read-only
        let meta2 = fs::metadata(&file2_path)?;
        assert!(meta2.permissions().readonly());

        Ok(())
    }

    #[test]
    fn test_platform_support_detection() {
        #[cfg(unix)]
        {
            assert!(supports_full_permissions());
            assert!(supports_any_permissions());
        }

        #[cfg(windows)]
        {
            assert!(!supports_full_permissions());
            assert!(supports_any_permissions());
        }

        #[cfg(not(any(unix, windows)))]
        {
            assert!(!supports_full_permissions());
            assert!(!supports_any_permissions());
        }
    }
}
