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
