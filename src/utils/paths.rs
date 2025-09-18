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
/// # Errors
///
/// Returns an error if the home directory cannot be determined
pub fn expand_tilde(path: &Path) -> Result<PathBuf> {
    if let Some(path_str) = path.to_str() {
        if path_str.starts_with("~/") || path_str == "~" {
            let home = dirs::home_dir().context("Could not find home directory")?;
            if path_str == "~" {
                Ok(home)
            } else {
                Ok(home.join(&path_str[2..]))
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

/// Checks if a path is within a base directory
///
/// # Errors
///
/// Returns an error if the base path cannot be canonicalized
pub fn is_within_directory(path: &Path, base: &Path) -> Result<bool> {
    let path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let base = base.canonicalize()?;
    Ok(path.starts_with(base))
}
