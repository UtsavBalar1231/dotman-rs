use std::path::{Path, PathBuf};
use shellexpand;

use crate::core::error::{DotmanError, Result};

/// Path utility functions
pub struct PathUtility;

impl PathUtility {
    /// Expand shell variables and tilde in path
    pub fn expand_path(path: &str) -> Result<PathBuf> {
        let expanded = shellexpand::full(path)
            .map_err(|e| DotmanError::path(format!("Failed to expand path '{}': {}", path, e)))?;
        
        Ok(PathBuf::from(expanded.as_ref()))
    }

    /// Normalize path by resolving .. and . components
    pub fn normalize_path(path: &Path) -> Result<PathBuf> {
        let canonical = path.canonicalize()
            .map_err(|e| DotmanError::path(format!("Failed to canonicalize path '{}': {}", path.display(), e)))?;
        
        Ok(canonical)
    }

    /// Check if path exists
    pub fn exists(path: &Path) -> bool {
        path.exists()
    }

    /// Check if path is under a parent directory
    pub fn is_under_directory(path: &Path, parent: &Path) -> Result<bool> {
        let normalized_path = Self::normalize_path(path)?;
        let normalized_parent = Self::normalize_path(parent)?;
        
        Ok(normalized_path.starts_with(normalized_parent))
    }

    /// Get relative path from base to target
    pub fn relative_path(base: &Path, target: &Path) -> Result<PathBuf> {
        let base_canonical = Self::normalize_path(base)?;
        let target_canonical = Self::normalize_path(target)?;
        
        target_canonical.strip_prefix(&base_canonical)
            .map(|p| p.to_path_buf())
            .map_err(|_| DotmanError::path(format!("Path '{}' is not under base '{}'", target.display(), base.display())))
    }

    /// Check if path requires elevated privileges
    pub fn requires_privileges(path: &Path) -> bool {
        // Common system directories that typically require root access
        let privileged_prefixes = [
            "/etc/",
            "/usr/",
            "/opt/",
            "/root/",
            "/var/log/",
            "/var/lib/",
            "/sys/",
            "/proc/",
            "/dev/",
            "/boot/",
        ];

        let path_str = path.to_string_lossy();
        privileged_prefixes.iter().any(|prefix| path_str.starts_with(prefix))
    }

    /// Safe join - prevents directory traversal attacks
    pub fn safe_join(base: &Path, child: &Path) -> Result<PathBuf> {
        // Ensure child is relative
        if child.is_absolute() {
            return Err(DotmanError::path("Child path must be relative".to_string()));
        }

        let joined = base.join(child);
        
        // Check for directory traversal without requiring paths to exist
        // by checking for ".." components that would escape the base
        let mut components = Vec::new();
        for component in child.components() {
            match component {
                std::path::Component::ParentDir => {
                    if components.is_empty() {
                        return Err(DotmanError::path("Path traversal attempt detected".to_string()));
                    }
                    components.pop();
                }
                std::path::Component::Normal(_) => {
                    components.push(component);
                }
                _ => {} // Ignore other components like . and prefix
            }
        }
        
        Ok(joined)
    }

    /// Get backup path for a given file
    pub fn get_backup_path(original: &Path, backup_dir: &Path) -> Result<PathBuf> {
        // Get relative path from root
        let relative = if original.is_absolute() {
            original.strip_prefix("/")
                .map_err(|_| DotmanError::path("Failed to make path relative".to_string()))?
        } else {
            original
        };
        
        Ok(backup_dir.join(relative))
    }

    /// Ensure parent directory exists
    pub async fn ensure_parent_exists(path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                tokio::fs::create_dir_all(parent).await
                    .map_err(|e| DotmanError::filesystem(format!("Failed to create parent directory: {}", e)))?;
            }
        }
        Ok(())
    }

    /// Generate temporary file path
    pub fn temp_path(prefix: &str, suffix: &str) -> PathBuf {
        let temp_dir = std::env::temp_dir();
        let filename = format!("{}-{}{}", prefix, uuid::Uuid::new_v4(), suffix);
        temp_dir.join(filename)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_expand_path() {
        // Test with environment variable
        std::env::set_var("TEST_VAR", "/test/path");
        let expanded = PathUtility::expand_path("$TEST_VAR/file.txt").unwrap();
        assert_eq!(expanded, PathBuf::from("/test/path/file.txt"));
        
        // Test with tilde (if HOME is set)
        if let Ok(home) = std::env::var("HOME") {
            let expanded = PathUtility::expand_path("~/file.txt").unwrap();
            assert_eq!(expanded, PathBuf::from(format!("{}/file.txt", home)));
        }
    }

    #[tokio::test]
    async fn test_path_operations() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        
        // Test exists
        assert!(PathUtility::exists(temp_path));
        
        // Test ensure parent exists
        let nested_file = temp_path.join("a/b/c/file.txt");
        PathUtility::ensure_parent_exists(&nested_file).await.unwrap();
        assert!(nested_file.parent().unwrap().exists());
    }

    #[test]
    fn test_privileges_detection() {
        assert!(PathUtility::requires_privileges(Path::new("/etc/passwd")));
        assert!(PathUtility::requires_privileges(Path::new("/usr/bin/test")));
        assert!(!PathUtility::requires_privileges(Path::new("/home/user/file.txt")));
        assert!(!PathUtility::requires_privileges(Path::new("./local/file.txt")));
    }

    #[test]
    fn test_safe_join() {
        let temp_dir = TempDir::new().unwrap();
        let base = temp_dir.path().join("safe/base");
        std::fs::create_dir_all(&base).unwrap();
        
        // Valid join
        let result = PathUtility::safe_join(&base, Path::new("subdir/file.txt")).unwrap();
        assert_eq!(result, base.join("subdir/file.txt"));
        
        // Should reject absolute paths
        assert!(PathUtility::safe_join(&base, Path::new("/etc/passwd")).is_err());
    }

    #[test]
    fn test_backup_path() {
        let original = Path::new("/home/user/.bashrc");
        let backup_dir = Path::new("/backup");
        
        let backup_path = PathUtility::get_backup_path(original, backup_dir).unwrap();
        assert_eq!(backup_path, PathBuf::from("/backup/home/user/.bashrc"));
    }
} 