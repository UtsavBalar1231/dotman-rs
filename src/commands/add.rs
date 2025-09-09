use crate::DotmanContext;
use crate::commands::context::CommandContext;
use crate::storage::{CachedHash, FileEntry};
use crate::utils::{expand_tilde, make_relative, should_ignore};
use anyhow::{Context, Result};
use colored::Colorize;
use rayon::prelude::*;
use std::path::{Path, PathBuf};

/// Execute the add command to stage files for tracking
///
/// # Errors
///
/// Returns an error if:
/// - The repository is not initialized
/// - A specified path does not exist (when not using force)
/// - Failed to read directory entries
/// - Failed to save the index
pub fn execute(ctx: &DotmanContext, paths: &[String], force: bool) -> Result<()> {
    ctx.ensure_initialized()?;

    let index_path = ctx.repo_path.join("index.bin");
    let mut index = ctx.load_index()?;

    let mut files_to_add = Vec::new();

    for path_str in paths {
        let path = expand_tilde(path_str);

        if !path.exists() {
            if !force {
                return Err(anyhow::anyhow!("Path does not exist: {}", path.display()));
            }
            super::print_warning(&format!("Skipping non-existent path: {}", path.display()));
            continue;
        }

        if path.is_file() {
            check_special_file_type(&path);
            files_to_add.push(path);
        } else if path.is_dir() {
            collect_files_from_dir(
                &path,
                &mut files_to_add,
                &ctx.config.tracking.ignore_patterns,
                ctx.config.tracking.follow_symlinks,
            )?;
        }
    }

    if files_to_add.is_empty() {
        super::print_info("No files to add");
        return Ok(());
    }

    let home = ctx.get_home_dir()?;

    let entries: Result<Vec<FileEntry>> = files_to_add
        .par_iter()
        .map(|path| {
            // Try to get existing cached hash from index
            let relative_path = make_relative(path, &home).ok();
            let cached_hash = relative_path
                .as_ref()
                .and_then(|rp| index.get_staged_entry(rp).or_else(|| index.get_entry(rp)))
                .and_then(|e| e.cached_hash.clone());
            create_file_entry_with_cache(path, &home, cached_hash.as_ref())
        })
        .collect();

    let entries = entries?;

    let mut added_count = 0;
    let mut updated_count = 0;

    for entry in entries {
        let is_tracked = index.get_entry(&entry.path).is_some();
        let is_staged = index.get_staged_entry(&entry.path).is_some();

        index.stage_entry(entry.clone());

        if is_tracked {
            updated_count += 1;
            println!("  {} {}", "modified:".yellow(), entry.path.display());
        } else if is_staged {
            updated_count += 1;
            println!("  {} {}", "updated:".yellow(), entry.path.display());
        } else {
            added_count += 1;
            println!("  {} {}", "added:".green(), entry.path.display());
        }
    }

    index.save_merge(&index_path)?;

    if added_count > 0 || updated_count > 0 {
        super::print_success(&format!(
            "Added {added_count} file(s), updated {updated_count} file(s)"
        ));
    } else {
        super::print_info("No changes made");
    }

    Ok(())
}

/// Collect all files from a directory recursively
///
/// # Errors
///
/// Returns an error if failed to read directory entries
fn collect_files_from_dir(
    dir: &Path,
    files: &mut Vec<PathBuf>,
    ignore_patterns: &[String],
    follow_symlinks: bool,
) -> Result<()> {
    for entry in walkdir::WalkDir::new(dir)
        .follow_links(follow_symlinks)
        .into_iter()
        .filter_entry(|e| !should_ignore(e.path(), ignore_patterns))
    {
        let entry =
            entry.with_context(|| format!("Failed to read directory: {}", dir.display()))?;
        if entry.file_type().is_file() {
            let file_path = entry.path().to_path_buf();
            check_special_file_type(&file_path);
            files.push(file_path);
        }
    }
    Ok(())
}

#[cfg(unix)]
#[allow(clippy::cast_precision_loss)]
fn check_special_file_type(path: &Path) {
    use std::os::unix::fs::FileTypeExt;

    if let Ok(metadata) = std::fs::metadata(path) {
        let file_type = metadata.file_type();

        {
            if file_type.is_block_device() {
                super::print_warning(&format!("Warning: {} is a block device", path.display()));
            } else if file_type.is_char_device() {
                super::print_warning(&format!(
                    "Warning: {} is a character device",
                    path.display()
                ));
            } else if file_type.is_fifo() {
                super::print_warning(&format!(
                    "Warning: {} is a named pipe (FIFO)",
                    path.display()
                ));
            } else if file_type.is_socket() {
                super::print_warning(&format!("Warning: {} is a socket", path.display()));
            }
        }

        if metadata.len() > 100_000_000 {
            #[allow(clippy::cast_precision_loss)]
            let size_mb = metadata.len() as f64 / 1_048_576.0;
            super::print_warning(&format!(
                "Warning: {} is very large ({:.2} MB)",
                path.display(),
                size_mb
            ));
        }

        if let Some(name) = path.file_name().and_then(|n| n.to_str())
            && (name.contains("password")
                || name.contains("secret")
                || name.contains("key")
                || name.contains(".pem")
                || name.contains(".key")
                || name.contains(".pfx"))
        {
            super::print_warning(&format!(
                "Warning: {} may contain sensitive information",
                path.display()
            ));
        }
    }
}

#[cfg(not(unix))]
fn check_special_file_type(path: &Path) {
    if let Ok(metadata) = std::fs::metadata(path) {
        if metadata.len() > 100_000_000 {
            #[allow(clippy::cast_precision_loss)]
            let size_mb = metadata.len() as f64 / 1_048_576.0;
            super::print_warning(&format!(
                "Warning: {} is very large ({:.2} MB)",
                path.display(),
                size_mb
            ));
        }

        if let Some(name) = path.file_name().and_then(|n| n.to_str())
            && (name.contains("password")
                || name.contains("secret")
                || name.contains("key")
                || name.contains(".pem")
                || name.contains(".key")
                || name.contains(".pfx"))
        {
            super::print_warning(&format!(
                "Warning: {} may contain sensitive information",
                path.display()
            ));
        }
    }
}

/// Create a `FileEntry` from a file path with optional cached hash
///
/// # Errors
///
/// Returns an error if:
/// - Failed to get file metadata
/// - Failed to hash the file
/// - Failed to make path relative
pub fn create_file_entry_with_cache(
    path: &Path,
    home: &Path,
    cached_hash: Option<&CachedHash>,
) -> Result<FileEntry> {
    let metadata = std::fs::metadata(path)
        .with_context(|| format!("Failed to get metadata for: {}", path.display()))?;

    let (hash, cache) = crate::storage::file_ops::hash_file(path, cached_hash)
        .with_context(|| format!("Failed to hash file: {}", path.display()))?;

    let modified = i64::try_from(
        metadata
            .modified()
            .context("Failed to get file modification time")?
            .duration_since(std::time::UNIX_EPOCH)
            .context("Invalid file modification time")?
            .as_secs(),
    )
    .context("File modification time too large")?;

    // Use the cross-platform permissions module
    let permissions = crate::utils::permissions::FilePermissions::from_path(path)?;
    let mode = permissions.mode();

    let relative_path = make_relative(path, home)
        .with_context(|| format!("Failed to make path relative: {}", path.display()))?;

    Ok(FileEntry {
        path: relative_path,
        hash,
        size: metadata.len(),
        modified,
        mode,
        cached_hash: Some(cache),
    })
}

/// Create a `FileEntry` from a file path (legacy compatibility)
///
/// # Errors
///
/// Returns an error if:
/// - Failed to get file metadata
/// - Failed to hash the file
/// - Failed to make path relative
pub fn create_file_entry(path: &Path, home: &Path) -> Result<FileEntry> {
    create_file_entry_with_cache(path, home, None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_create_file_entry() -> Result<()> {
        let dir = tempdir()?;
        let file_path = dir.path().join("test.txt");
        std::fs::write(&file_path, "test content")?;

        let entry = create_file_entry(&file_path, dir.path())?;

        assert_eq!(entry.path, PathBuf::from("test.txt"));
        assert!(!entry.hash.is_empty());
        assert_eq!(entry.size, 12);
        assert!(entry.modified > 0);
        assert!(entry.cached_hash.is_some());

        // Test that cache is populated correctly
        let cache = entry.cached_hash.as_ref().unwrap();
        assert_eq!(cache.hash, entry.hash);
        assert_eq!(cache.size_at_hash, entry.size);
        assert_eq!(cache.mtime_at_hash, entry.modified);

        Ok(())
    }

    #[test]
    fn test_create_file_entry_with_cache() -> Result<()> {
        let dir = tempdir()?;
        let file_path = dir.path().join("cached.txt");
        std::fs::write(&file_path, "cached content")?;

        // First call without cache
        let entry1 = create_file_entry(&file_path, dir.path())?;
        assert!(entry1.cached_hash.is_some());

        // Second call with cache from first entry
        let entry2 =
            create_file_entry_with_cache(&file_path, dir.path(), entry1.cached_hash.as_ref())?;

        // Should have same hash (file unchanged)
        assert_eq!(entry2.hash, entry1.hash);
        assert_eq!(entry2.cached_hash.as_ref().unwrap().hash, entry1.hash);

        Ok(())
    }
}
