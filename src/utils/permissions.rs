use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

/// Permission bit for setuid (set user ID on execution).
///
/// When set on an executable file, the process runs with the permissions
/// of the file's owner rather than the user who executed it.
/// **Security risk**: Can be exploited for privilege escalation.
pub const SETUID: u32 = 0o4000;

/// Permission bit for setgid (set group ID on execution).
///
/// When set on an executable file, the process runs with the permissions
/// of the file's group. On directories, new files inherit the directory's group.
/// **Security risk**: Can be exploited for privilege escalation.
pub const SETGID: u32 = 0o2000;

/// Permission bit for sticky bit.
///
/// On directories, prevents users from deleting files they don't own.
/// On some systems, affects caching behavior for executables.
/// **Security risk**: Generally lower risk but can affect system behavior.
pub const STICKY: u32 = 0o1000;

/// Mask for all dangerous permission bits (setuid | setgid | sticky).
pub const DANGEROUS_BITS: u32 = SETUID | SETGID | STICKY; // 0o7000

/// Mask for safe permission bits (rwxrwxrwx).
///
/// This mask preserves only standard read, write, and execute permissions
/// for owner, group, and others.
pub const SAFE_MASK: u32 = 0o777;

/// Cross-platform file permissions handling
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FilePermissions {
    /// Raw permission mode bits (Unix-style, even on other platforms)
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

    /// Create a sanitized copy with dangerous bits stripped.
    ///
    /// Returns a new `FilePermissions` with setuid, setgid, and sticky bits removed,
    /// preserving only standard rwxrwxrwx permissions.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dotman::utils::permissions::{FilePermissions, SETUID, SETGID, SAFE_MASK};
    /// let perms = FilePermissions::from_mode(0o4755); // rwsr-xr-x
    /// let safe = perms.sanitized();
    /// assert_eq!(safe.mode(), 0o755); // rwxr-xr-x
    /// ```
    #[must_use]
    pub const fn sanitized(&self) -> Self {
        Self {
            mode: self.mode & SAFE_MASK,
        }
    }

    /// Strip dangerous permission bits in-place.
    ///
    /// Removes setuid, setgid, and sticky bits, preserving only standard
    /// rwxrwxrwx permissions.
    pub const fn strip_dangerous_bits(&mut self) {
        self.mode &= SAFE_MASK;
    }

    /// Check if this permission set contains any dangerous bits.
    ///
    /// Returns `true` if setuid, setgid, or sticky bits are set.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dotman::utils::permissions::FilePermissions;
    /// assert!(FilePermissions::from_mode(0o4755).has_dangerous_bits()); // setuid
    /// assert!(FilePermissions::from_mode(0o2755).has_dangerous_bits()); // setgid
    /// assert!(FilePermissions::from_mode(0o1755).has_dangerous_bits()); // sticky
    /// assert!(!FilePermissions::from_mode(0o755).has_dangerous_bits()); // safe
    /// ```
    #[must_use]
    pub const fn has_dangerous_bits(&self) -> bool {
        (self.mode & DANGEROUS_BITS) != 0
    }

    /// Get which dangerous bits are set.
    ///
    /// Returns a vector of strings describing which dangerous bits are present.
    /// Empty vector if no dangerous bits are set.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dotman::utils::permissions::FilePermissions;
    /// let perms = FilePermissions::from_mode(0o4755);
    /// let dangerous = perms.get_dangerous_bits();
    /// assert_eq!(dangerous, vec!["setuid (0o4000)"]);
    /// ```
    #[must_use]
    pub fn get_dangerous_bits(&self) -> Vec<String> {
        let mut bits = Vec::new();
        if (self.mode & SETUID) != 0 {
            bits.push(format!("setuid (0o{SETUID:o})"));
        }
        if (self.mode & SETGID) != 0 {
            bits.push(format!("setgid (0o{SETGID:o})"));
        }
        if (self.mode & STICKY) != 0 {
            bits.push(format!("sticky (0o{STICKY:o})"));
        }
        bits
    }

    /// Read permissions from a file
    ///
    /// # Arguments
    ///
    /// * `path` - The file path to read permissions from
    /// * `strip_dangerous` - If `true`, strips setuid/setgid/sticky bits for security
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Failed to read file metadata
    ///
    /// # Security
    ///
    /// When `strip_dangerous` is `true`, dangerous permission bits (setuid, setgid, sticky)
    /// are automatically removed. This is the recommended setting for most use cases to
    /// prevent privilege escalation attacks when files are restored.
    pub fn from_path(path: &Path, strip_dangerous: bool) -> Result<Self> {
        let metadata = fs::metadata(path)
            .with_context(|| format!("Failed to read metadata for: {}", path.display()))?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            let mut perms = Self::from_mode(metadata.mode());
            if strip_dangerous {
                perms.strip_dangerous_bits();
            }
            Ok(perms)
        }

        #[cfg(windows)]
        {
            // On Windows, we'll store a simplified permission model
            // Read-only flag is the main permission we can preserve
            // Windows doesn't have setuid/setgid/sticky, so strip_dangerous is a no-op
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
            // No dangerous bits on non-Unix, so strip_dangerous is a no-op
            let _ = strip_dangerous; // Suppress unused parameter warning
            Ok(Self::from_mode(0o644))
        }
    }

    /// Apply permissions to a file if `preserve_permissions` is enabled
    ///
    /// # Arguments
    ///
    /// * `path` - The file path to apply permissions to
    /// * `preserve_permissions` - If `true`, applies stored permissions
    /// * `allow_dangerous` - If `true`, allows dangerous bits; if `false`, strips them
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Failed to set file permissions (only on platforms that support it)
    ///
    /// # Security
    ///
    /// When `allow_dangerous` is `false`, dangerous permission bits are stripped before
    /// applying. This is the recommended setting to prevent privilege escalation attacks.
    pub fn apply_to_path(
        &self,
        path: &Path,
        preserve_permissions: bool,
        allow_dangerous: bool,
    ) -> Result<()> {
        if !preserve_permissions {
            return Ok(());
        }

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            // Sanitize permissions if dangerous bits should be stripped
            let mode = if allow_dangerous {
                self.mode
            } else {
                self.mode & SAFE_MASK
            };
            let permissions = fs::Permissions::from_mode(mode);
            fs::set_permissions(path, permissions)
                .with_context(|| format!("Failed to set permissions for: {}", path.display()))?;
        }

        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;

            // On Windows, we can only reliably set the read-only flag
            // Check if the stored mode indicates read-only (no write bits)
            // Windows doesn't have setuid/setgid/sticky, so allow_dangerous is a no-op
            let _ = allow_dangerous; // Suppress unused parameter warning
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
            let _ = (path, allow_dangerous); // Suppress unused variable warnings
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
/// # Arguments
///
/// * `src` - Source file path
/// * `dst` - Destination file path
/// * `preserve_permissions` - If `true`, copies permissions from source to destination
/// * `strip_dangerous` - If `true`, strips dangerous bits when reading permissions
///
/// # Errors
///
/// Returns an error if:
/// - Failed to copy the file
/// - Failed to preserve permissions (if enabled and supported)
///
/// # Security
///
/// When `strip_dangerous` is `true`, dangerous permission bits (setuid, setgid, sticky)
/// are removed when copying permissions. This is the recommended setting.
pub fn copy_with_permissions(
    src: &Path,
    dst: &Path,
    preserve_permissions: bool,
    strip_dangerous: bool,
) -> Result<()> {
    // Copy the file content
    fs::copy(src, dst)
        .with_context(|| format!("Failed to copy {} to {}", src.display(), dst.display()))?;

    // Preserve permissions if requested
    if preserve_permissions {
        let permissions = FilePermissions::from_path(src, strip_dangerous)?;
        permissions.apply_to_path(dst, true, !strip_dangerous)?;
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
