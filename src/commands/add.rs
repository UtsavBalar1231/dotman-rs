use crate::storage::{FileEntry, index::Index};
use crate::utils::{expand_tilde, hash::hash_file, make_relative, should_ignore};
use crate::{DotmanContext, INDEX_FILE};
use anyhow::{Context, Result};
use colored::Colorize;
use rayon::prelude::*;
use std::path::{Path, PathBuf};

pub fn execute(ctx: &DotmanContext, paths: &[String], force: bool) -> Result<()> {
    ctx.ensure_repo_exists()?;

    let index_path = ctx.repo_path.join(INDEX_FILE);
    let mut index = Index::load(&index_path)?;

    // Expand paths and collect files to add
    let mut files_to_add = Vec::new();

    for path_str in paths {
        let path = expand_tilde(path_str);

        if !path.exists() {
            if !force {
                anyhow::bail!("Path does not exist: {}", path.display());
            } else {
                super::print_warning(&format!("Skipping non-existent path: {}", path.display()));
                continue;
            }
        }

        if path.is_file() {
            // Check for special file types and warn
            check_special_file_type(&path);
            files_to_add.push(path);
        } else if path.is_dir() {
            // Recursively add all files in directory
            collect_files_from_dir(
                &path,
                &mut files_to_add,
                &ctx.config.tracking.ignore_patterns,
            )?;
        }
    }

    if files_to_add.is_empty() {
        super::print_info("No files to add");
        return Ok(());
    }

    // Get home directory for making paths relative
    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;

    // Process files in parallel
    let entries: Result<Vec<FileEntry>> = files_to_add
        .par_iter()
        .map(|path| create_file_entry(path, &home))
        .collect();

    let entries = entries?;

    // Add entries to index
    let mut added_count = 0;
    let mut updated_count = 0;

    for entry in entries {
        let is_update = index.get_entry(&entry.path).is_some();
        index.add_entry(entry.clone());

        if is_update {
            updated_count += 1;
            println!("  {} {}", "modified:".yellow(), entry.path.display());
        } else {
            added_count += 1;
            println!("  {} {}", "added:".green(), entry.path.display());
        }
    }

    // Save index - use save_merge to handle concurrent adds properly
    index.save_merge(&index_path)?;

    if added_count > 0 || updated_count > 0 {
        super::print_success(&format!(
            "Added {} file(s), updated {} file(s)",
            added_count, updated_count
        ));
    } else {
        super::print_info("No changes made");
    }

    Ok(())
}

fn collect_files_from_dir(
    dir: &Path,
    files: &mut Vec<PathBuf>,
    ignore_patterns: &[String],
) -> Result<()> {
    for entry in walkdir::WalkDir::new(dir)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| !should_ignore(e.path(), ignore_patterns))
    {
        let entry = entry?;
        if entry.file_type().is_file() {
            let file_path = entry.path().to_path_buf();
            check_special_file_type(&file_path);
            files.push(file_path);
        }
    }
    Ok(())
}

fn check_special_file_type(path: &Path) {
    use std::os::unix::fs::FileTypeExt;

    if let Ok(metadata) = std::fs::metadata(path) {
        let file_type = metadata.file_type();

        #[cfg(unix)]
        {
            if file_type.is_block_device() {
                super::print_warning(&format!("⚠️  {} is a block device", path.display()));
            } else if file_type.is_char_device() {
                super::print_warning(&format!("⚠️  {} is a character device", path.display()));
            } else if file_type.is_fifo() {
                super::print_warning(&format!("⚠️  {} is a named pipe (FIFO)", path.display()));
            } else if file_type.is_socket() {
                super::print_warning(&format!("⚠️  {} is a socket", path.display()));
            }
        }

        // Check for large files
        if metadata.len() > 100_000_000 {
            // 100MB
            super::print_warning(&format!(
                "⚠️  {} is very large ({:.2} MB)",
                path.display(),
                metadata.len() as f64 / 1_048_576.0
            ));
        }

        // Check for sensitive file patterns
        if let Some(name) = path.file_name().and_then(|n| n.to_str())
            && (name.contains("password")
                || name.contains("secret")
                || name.contains("key")
                || name.contains(".pem")
                || name.contains(".key")
                || name.contains(".pfx"))
        {
            super::print_warning(&format!(
                "⚠️  {} may contain sensitive information",
                path.display()
            ));
        }
    }
}

pub fn create_file_entry(path: &Path, home: &Path) -> Result<FileEntry> {
    let metadata = std::fs::metadata(path)
        .with_context(|| format!("Failed to get metadata for: {}", path.display()))?;

    let hash = hash_file(path)?;

    let modified = metadata
        .modified()?
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs() as i64;

    #[cfg(unix)]
    let mode = {
        use std::os::unix::fs::MetadataExt;
        metadata.mode()
    };

    #[cfg(not(unix))]
    let mode = 0o644;

    // Store paths relative to home directory for portability
    let relative_path = make_relative(path, home)?;

    Ok(FileEntry {
        path: relative_path,
        hash,
        size: metadata.len(),
        modified,
        mode,
    })
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

        // Use temp dir as home for testing
        let entry = create_file_entry(&file_path, dir.path())?;

        // Path should be relative to "home" (temp dir in this case)
        assert_eq!(entry.path, PathBuf::from("test.txt"));
        assert!(!entry.hash.is_empty());
        assert_eq!(entry.size, 12);
        assert!(entry.modified > 0);

        Ok(())
    }
}
