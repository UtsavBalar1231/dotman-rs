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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_normalize_to_home_relative() {
        let home = dirs::home_dir().unwrap();
        let test_path = home.join("test/file.txt");

        let result = normalize_to_home_relative(&test_path).unwrap();
        assert_eq!(result, PathBuf::from("test/file.txt"));

        let relative_path = PathBuf::from("test/file.txt");
        let result = normalize_to_home_relative(&relative_path).unwrap();
        assert_eq!(result, relative_path);
    }

    #[test]
    fn test_normalize_to_relative() {
        let base = PathBuf::from("/home/user");
        let path = PathBuf::from("/home/user/documents/file.txt");

        let result = normalize_to_relative(&path, &base);
        assert_eq!(result, PathBuf::from("documents/file.txt"));

        let relative_path = PathBuf::from("documents/file.txt");
        let result = normalize_to_relative(&relative_path, &base);
        assert_eq!(result, relative_path);
    }

    #[test]
    fn test_ensure_parent_dirs() {
        let temp_dir = TempDir::new().unwrap();
        let nested_file = temp_dir.path().join("a/b/c/file.txt");

        ensure_parent_dirs(&nested_file).unwrap();
        assert!(nested_file.parent().unwrap().exists());
    }

    #[test]
    fn test_expand_tilde() {
        let home = dirs::home_dir().unwrap();

        let tilde_path = PathBuf::from("~/documents");
        let result = expand_tilde(&tilde_path).unwrap();
        assert_eq!(result, home.join("documents"));

        let just_tilde = PathBuf::from("~");
        let result = expand_tilde(&just_tilde).unwrap();
        assert_eq!(result, home);

        let no_tilde = PathBuf::from("/absolute/path");
        let result = expand_tilde(&no_tilde).unwrap();
        assert_eq!(result, no_tilde);
    }

    #[test]
    fn test_make_absolute() {
        let absolute = PathBuf::from("/absolute/path");
        let result = make_absolute(&absolute).unwrap();
        assert_eq!(result, absolute);

        let relative = PathBuf::from("relative/path");
        let result = make_absolute(&relative).unwrap();
        assert!(result.is_absolute());
        assert!(result.ends_with("relative/path"));
    }

    #[test]
    fn test_is_within_directory() {
        let temp_dir = TempDir::new().unwrap();
        let base = temp_dir.path();
        let within = base.join("subdir/file.txt");

        // Create the directories so canonicalize works
        fs::create_dir_all(within.parent().unwrap()).unwrap();
        fs::write(&within, "test").unwrap();

        assert!(is_within_directory(&within, base).unwrap());

        let outside = PathBuf::from("/tmp/outside");
        assert!(!is_within_directory(&outside, base).unwrap_or(false));
    }
}
