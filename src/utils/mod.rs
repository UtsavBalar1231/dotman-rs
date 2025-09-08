pub mod commit;
pub mod compress;
pub mod pager;
pub mod serialization;

use anyhow::Result;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Expands a path starting with `~` to the user's home directory.
#[must_use]
pub fn expand_tilde(path: &str) -> PathBuf {
    if path.starts_with("~/")
        && let Some(home) = dirs::home_dir()
    {
        return home.join(&path[2..]);
    }
    PathBuf::from(path)
}

/// Make `path` relative to `base` if possible, otherwise return `path` as is.
///
/// # Errors
/// If `base` is not a prefix of `path`, an error is returned.
pub fn make_relative(path: &Path, base: &Path) -> Result<PathBuf> {
    path.strip_prefix(base)
        .map(Path::to_path_buf)
        .or_else(|_| Ok(path.to_path_buf()))
}

/// Walks a directory and returns all file paths that pass the provided filter function.
///
/// # Errors
/// Returns an error if any entry cannot be accessed.
pub fn walk_dir_filtered<F>(dir: &Path, filter: F, follow_symlinks: bool) -> Result<Vec<PathBuf>>
where
    F: Fn(&Path) -> bool,
{
    let mut paths = Vec::new();

    for entry in WalkDir::new(dir)
        .follow_links(follow_symlinks)
        .into_iter()
        .filter_entry(|e| filter(e.path()))
    {
        let entry = entry?;
        if entry.file_type().is_file() {
            paths.push(entry.path().to_path_buf());
        }
    }

    Ok(paths)
}

/// Determines if a given path should be ignored based on provided patterns.
#[must_use]
pub fn should_ignore(path: &Path, patterns: &[String]) -> bool {
    let path_str = path.to_string_lossy();

    for pattern in patterns {
        // Handle directory patterns (ending with /)
        if pattern.ends_with('/') {
            let dir_name = &pattern[..pattern.len() - 1];
            // or if the path contains this directory
            if path.components().any(|c| c.as_os_str() == dir_name) {
                return true;
            }
            // Also check if path starts with or contains the directory pattern
            if path_str.contains(&format!("/{dir_name}/"))
                || path_str.starts_with(&format!("{dir_name}/"))
                || path_str == dir_name
            {
                return true;
            }
        } else if pattern.starts_with('*') && pattern.ends_with('*') {
            // Contains pattern
            let search = &pattern[1..pattern.len() - 1];
            if path_str.contains(search) {
                return true;
            }
        } else if let Some(suffix) = pattern.strip_prefix('*') {
            // Ends with pattern
            if path_str.ends_with(suffix) {
                return true;
            }
        } else if pattern.ends_with('*') {
            // Starts with pattern
            let prefix = &pattern[..pattern.len() - 1];
            if path_str.starts_with(prefix) {
                return true;
            }
        } else {
            // Exact match or path component match
            if path_str == pattern.as_str()
                || path.components().any(|c| c.as_os_str() == pattern.as_str())
            {
                return true;
            }
        }
    }

    false
}

/// Formats a file size in bytes into a human-readable string with appropriate units.
#[must_use]
#[allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
pub fn format_size(size: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = size as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{} {}", size.round() as u64, UNITS[unit_index])
    } else {
        format!("{:.2} {}", size, UNITS[unit_index])
    }
}

/// Returns the current timestamp as seconds since the Unix epoch.
#[must_use]
pub fn get_current_timestamp() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX))
        .unwrap_or(0)
}

/// Retrieves the current system username, falling back to "unknown" if not found.
#[must_use]
pub fn get_current_user() -> String {
    std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "unknown".to_string())
}

/// Constructs the current user string based on the provided configuration.
#[must_use]
pub fn get_current_user_with_config(config: &crate::config::Config) -> String {
    match (&config.user.name, &config.user.email) {
        (Some(name), Some(email)) => {
            // Format as "Name <email>" like git does
            format!("{name} <{email}>")
        }
        (Some(name), None) => {
            // Only name is set
            name.clone()
        }
        (None, Some(email)) => {
            // Only email is set, use system username with email
            let username = get_current_user();
            format!("{username} <{email}>")
        }
        (None, None) => {
            // Neither is set, fall back to system username
            get_current_user()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_expand_tilde() {
        let expanded = expand_tilde("~/test");
        assert!(expanded.starts_with(dirs::home_dir().unwrap()));

        let no_tilde = expand_tilde("/absolute/path");
        assert_eq!(no_tilde, PathBuf::from("/absolute/path"));
    }

    #[test]
    fn test_should_ignore() {
        let patterns = vec![
            "*.swp".to_string(),
            ".git".to_string(),
            "node_modules".to_string(),
            "*cache*".to_string(),
        ];

        assert!(should_ignore(Path::new("file.swp"), &patterns));
        assert!(should_ignore(Path::new(".git"), &patterns));
        assert!(should_ignore(Path::new("node_modules"), &patterns));
        assert!(should_ignore(Path::new("some/cache/dir"), &patterns));
        assert!(!should_ignore(Path::new("normal_file.txt"), &patterns));
    }

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(512), "512 B");
        assert_eq!(format_size(1024), "1.00 KB");
        assert_eq!(format_size(1536), "1.50 KB");
        assert_eq!(format_size(1_048_576), "1.00 MB");
        assert_eq!(format_size(5_242_880), "5.00 MB");
    }

    #[test]
    fn test_walk_dir_filtered() -> Result<()> {
        let dir = tempdir()?;

        // Create test files
        std::fs::write(dir.path().join("file1.txt"), "content1")?;
        std::fs::write(dir.path().join("file2.md"), "content2")?;
        std::fs::create_dir(dir.path().join("subdir"))?;
        std::fs::write(dir.path().join("subdir/file3.txt"), "content3")?;

        // Walk and filter for .txt files
        let filter = |p: &Path| p.extension().is_none_or(|ext| ext == "txt") || p.is_dir();

        let files = walk_dir_filtered(dir.path(), filter, false)?;
        assert_eq!(files.len(), 2);
        assert!(files.iter().all(|p| p.extension().unwrap() == "txt"));

        Ok(())
    }

    #[test]
    fn test_get_current_user_with_config_both_set() {
        let mut config = crate::config::Config::default();
        config.user.name = Some("John Doe".to_string());
        config.user.email = Some("john@example.com".to_string());

        let user = get_current_user_with_config(&config);
        assert_eq!(user, "John Doe <john@example.com>");
    }

    #[test]
    fn test_get_current_user_with_config_only_name() {
        let mut config = crate::config::Config::default();
        config.user.name = Some("Jane Smith".to_string());
        config.user.email = None;

        let user = get_current_user_with_config(&config);
        assert_eq!(user, "Jane Smith");
    }

    #[test]
    fn test_get_current_user_with_config_only_email() {
        let mut config = crate::config::Config::default();
        config.user.name = None;
        config.user.email = Some("user@example.com".to_string());

        let user = get_current_user_with_config(&config);
        let system_user = get_current_user();
        assert_eq!(user, format!("{system_user} <user@example.com>"));
    }

    #[test]
    fn test_get_current_user_with_config_none_set() {
        let config = crate::config::Config::default();

        let user = get_current_user_with_config(&config);
        let system_user = get_current_user();
        assert_eq!(user, system_user);
    }
}
