use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

/// Normalizes a path to be relative to the home directory
///
/// # Errors
///
/// Returns an error if the home directory cannot be determined
pub fn normalize_to_home_relative(path: &Path) -> Result<PathBuf> {
    let home = dirs::home_dir().context("Could not find home directory")?;

    if path.is_absolute() {
        Ok(path.strip_prefix(&home).unwrap_or(path).to_path_buf())
    } else {
        Ok(path.to_path_buf())
    }
}

/// Normalizes a path to be relative to a base directory
#[must_use]
pub fn normalize_to_relative(path: &Path, base: &Path) -> PathBuf {
    if path.is_absolute() {
        path.strip_prefix(base).unwrap_or(path).to_path_buf()
    } else {
        path.to_path_buf()
    }
}

/// Ensures parent directories exist for a given path
///
/// # Errors
///
/// Returns an error if the parent directories cannot be created
pub fn ensure_parent_dirs(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent()
        && !parent.exists()
    {
        fs::create_dir_all(parent).with_context(|| {
            format!("Failed to create parent directories for {}", path.display())
        })?;
    }
    Ok(())
}

/// Expands tilde in path to home directory
///
/// # Security
///
/// This function detects path traversal attempts like `~/../../../etc/passwd`
/// and rejects them. For full security validation, use `validate_path_security()`
/// after expansion.
///
/// # Errors
///
/// Returns an error if:
/// - The home directory cannot be determined
/// - Path traversal is detected in tilde expansion
pub fn expand_tilde(path: &Path) -> Result<PathBuf> {
    if let Some(path_str) = path.to_str() {
        if path_str.starts_with("~/") || path_str == "~" {
            let home = dirs::home_dir().context("Could not find home directory")?;
            if path_str == "~" {
                Ok(home)
            } else {
                // Security: Prevent tilde bypass like "~/../../../etc"
                let remainder = &path_str[2..];
                if remainder.starts_with("..") {
                    return Err(anyhow::anyhow!(
                        "Path traversal detected in tilde expansion: {path_str}\n\
                        This pattern is not allowed for security reasons."
                    ));
                }
                Ok(home.join(remainder))
            }
        } else {
            Ok(path.to_path_buf())
        }
    } else {
        Ok(path.to_path_buf())
    }
}

/// Makes a path absolute, resolving relative paths from current directory
///
/// # Errors
///
/// Returns an error if the current directory cannot be determined
pub fn make_absolute(path: &Path) -> Result<PathBuf> {
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        let current_dir = std::env::current_dir()?;
        Ok(current_dir.join(path))
    }
}

/// Checks if a path is within a base directory after canonicalization
///
/// This function properly handles:
/// - Non-existent paths (canonicalizes parent directory)
/// - Symlinks (resolves them before checking)
/// - Relative paths (makes them absolute first)
///
/// # Errors
///
/// Returns an error if:
/// - The base path cannot be canonicalized
/// - Path canonicalization reveals security issues
/// - The parent directory does not exist for non-existent paths
pub fn is_within_directory(path: &Path, base: &Path) -> Result<bool> {
    // Canonicalize the base (must exist)
    let canonical_base = base
        .canonicalize()
        .with_context(|| format!("Failed to canonicalize base directory: {}", base.display()))?;

    // For the path, try to canonicalize if it exists
    // If it doesn't exist, canonicalize the parent and append the filename
    let canonical_path = if path.exists() {
        path.canonicalize()
            .with_context(|| format!("Failed to canonicalize path: {}", path.display()))?
    } else if let Some(parent) = path.parent() {
        if parent.as_os_str().is_empty() || parent == Path::new(".") {
            // Relative path with no parent, use current dir
            let current_dir = std::env::current_dir()?;
            current_dir.join(path.file_name().unwrap_or(path.as_os_str()))
        } else if parent.exists() {
            let canonical_parent = parent
                .canonicalize()
                .with_context(|| format!("Failed to canonicalize parent: {}", parent.display()))?;
            canonical_parent.join(path.file_name().unwrap_or(path.as_os_str()))
        } else {
            // Parent doesn't exist either, can't validate
            return Err(anyhow::anyhow!(
                "Cannot validate path: parent directory does not exist: {}",
                parent.display()
            ));
        }
    } else {
        // No parent component, treat as relative to current dir
        let current_dir = std::env::current_dir()?;
        current_dir.join(path)
    };

    Ok(canonical_path.starts_with(&canonical_base))
}

/// Validates a path against security policies
///
/// This function performs comprehensive path validation:
/// 1. Expands tilde if present
/// 2. Converts to absolute path
/// 3. Checks path is within allowed directories
/// 4. Prevents path traversal attacks
/// 5. Validates against symlink escapes
///
/// # Arguments
///
/// * `path` - The path to validate
/// * `allowed_dirs` - List of allowed base directories
/// * `enforce` - If true, strictly enforce validation; if false, only warn
///
/// # Errors
///
/// Returns an error if:
/// - Path is outside allowed directories (when enforce=true)
/// - Path validation fails
/// - Configuration is invalid
pub fn validate_path_security(
    path: &Path,
    allowed_dirs: &[PathBuf],
    enforce: bool,
) -> Result<PathBuf> {
    // First, expand tilde if present (this also checks for ~/../ patterns)
    let expanded = expand_tilde(path)?;

    // Convert to absolute path if relative
    let absolute = make_absolute(&expanded)?;

    // Check against each allowed directory
    let mut is_allowed = false;
    for allowed_dir in allowed_dirs {
        // Expand tilde in allowed directory
        let allowed_expanded = expand_tilde(allowed_dir)?;

        // Check if path is within this allowed directory
        match is_within_directory(&absolute, &allowed_expanded) {
            Ok(true) => {
                is_allowed = true;
                break;
            }
            Ok(false) => {}
            Err(_) if !enforce => {
                // In non-enforcement mode, allow paths if check fails
                // (for backwards compatibility)
                is_allowed = true;
                break;
            }
            Err(e) => return Err(e),
        }
    }

    if !is_allowed {
        if enforce {
            let allowed_list = allowed_dirs
                .iter()
                .map(|d| d.display().to_string())
                .collect::<Vec<_>>()
                .join("\n  - ");
            return Err(anyhow::anyhow!(
                "Path '{}' is outside allowed directories.\n\
                \n\
                Allowed directories:\n  - {}\n\
                \n\
                To track files in additional directories, edit ~/.config/dotman/config:\n\
                \n\
                [security]\n\
                allowed_directories = ['~', '/your/directory']\n\
                \n\
                WARNING: Only add directories you trust and control.",
                absolute.display(),
                allowed_list
            ));
        }
        // Warn but allow (for backwards compatibility)
        eprintln!(
            "Warning: Path '{}' is outside typical allowed directories",
            absolute.display()
        );
    }

    Ok(absolute)
}

/// Validates and normalizes a path for dotman operations
///
/// This function combines path validation with normalization to produce
/// a safe, home-relative path ready for storage in the index.
///
/// The process:
/// 1. Validates path security (via `validate_path_security`)
/// 2. Normalizes to be relative to home directory
///
/// # Arguments
///
/// * `path` - The path to validate and normalize
/// * `home` - The home directory (not currently used, kept for API compatibility)
/// * `allowed_dirs` - List of allowed base directories
/// * `enforce` - If true, strictly enforce validation; if false, only warn
///
/// # Errors
///
/// Returns an error if path validation fails
pub fn validate_and_normalize_path(
    path: &Path,
    _home: &Path,
    allowed_dirs: &[PathBuf],
    enforce: bool,
) -> Result<PathBuf> {
    // Validate security
    let validated = validate_path_security(path, allowed_dirs, enforce)?;

    // Normalize to relative path
    normalize_to_home_relative(&validated)
}
