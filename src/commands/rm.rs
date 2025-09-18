use crate::storage::index::Index;
use crate::{DotmanContext, INDEX_FILE};
use anyhow::{Context, Result};
use colored::Colorize;
use glob::Pattern;
use std::fs;
use std::path::{Path, PathBuf};

/// Execute rm command - remove files from tracking (the index)
///
/// Similar to git rm, this removes files from being tracked. With --cached,
/// only removes from index. Without --cached, removes from both index and
/// working directory (but we protect against data loss).
///
/// # Errors
///
/// Returns an error if:
/// - The repository is not initialized
/// - No files match the specified patterns
/// - File operations fail
/// - Index update fails
#[allow(clippy::fn_params_excessive_bools)]
pub fn execute(
    ctx: &DotmanContext,
    paths: &[String],
    cached: bool,
    force: bool,
    recursive: bool,
    dry_run: bool,
) -> Result<()> {
    ctx.check_repo_initialized()?;

    if dry_run {
        super::print_info("Dry run mode - no files will be removed");
    }

    let index_path = ctx.repo_path.join(INDEX_FILE);
    let mut index = Index::load(&index_path)?;

    let mut removed_count = 0;
    let mut not_found_count = 0;

    // Get home directory for making paths relative
    let home = dirs::home_dir().context("Could not find home directory")?;

    // Expand paths with glob patterns and recursive directory handling
    let mut expanded_paths = Vec::new();

    for path_str in paths {
        if path_str.contains('*') || path_str.contains('?') || path_str.contains('[') {
            // Handle glob pattern
            if let Ok(pattern) = Pattern::new(path_str) {
                // Match against files in index
                for indexed_path in index.entries.keys() {
                    if pattern.matches(&indexed_path.to_string_lossy()) {
                        expanded_paths.push(indexed_path.clone());
                    }
                }
            } else {
                super::print_warning(&format!("Invalid glob pattern: {path_str}"));
            }
        } else {
            let path = PathBuf::from(path_str);

            if recursive && path.is_dir() {
                // Add all files in directory recursively
                expand_directory_recursive(&path, &mut expanded_paths)?;
            } else {
                expanded_paths.push(path);
            }
        }
    }

    // Remove duplicates
    expanded_paths.sort();
    expanded_paths.dedup();

    for path in expanded_paths {
        let index_path = if path.is_absolute() {
            path.strip_prefix(&home).unwrap_or(&path).to_path_buf()
        } else {
            path.clone()
        };

        let in_index = index.get_entry(&index_path).is_some();

        if !in_index && !force {
            super::print_warning(&format!("File not tracked: {}", path.display()));
            not_found_count += 1;
            continue;
        }

        if dry_run {
            println!(
                "  {} {} (dry run)",
                "would remove:".yellow(),
                path.display()
            );
            removed_count += 1;
            continue;
        }

        if index.remove_entry(&index_path).is_some() {
            println!("  {} {}", "removed:".red(), path.display());
            removed_count += 1;
        }

        // When not using --cached, the traditional git behavior would delete
        // the file from the working directory. However, for safety in a dotfiles
        // manager, we NEVER delete actual files from the user's filesystem.
        // We only remove them from tracking.
        if !cached {
            // In a future implementation, we might add an interactive prompt
            // or a --force flag to actually delete files, but for now we
            // prioritize data safety over git compatibility.
        }
    }

    // Save updated index (only if not in dry run mode)
    if removed_count > 0 && !dry_run {
        index.save(&index_path)?;
        if cached {
            super::print_success(&format!(
                "Removed {removed_count} file(s) from index (files unchanged on disk)"
            ));
        } else {
            super::print_success(&format!("Removed {removed_count} file(s) from tracking"));
        }
    } else if removed_count > 0 && dry_run {
        super::print_success(&format!("Would remove {removed_count} file(s) (dry run)"));
    }

    if not_found_count > 0 {
        super::print_info(&format!("{not_found_count} file(s) were not tracked"));
    }

    Ok(())
}

fn expand_directory_recursive(dir: &Path, paths: &mut Vec<PathBuf>) -> Result<()> {
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                // Recursively expand subdirectories
                expand_directory_recursive(&path, paths)?;
            } else {
                paths.push(path);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::storage::FileEntry;
    use std::fs;
    use tempfile::tempdir;

    fn setup_test_context() -> Result<(tempfile::TempDir, DotmanContext)> {
        let temp = tempdir()?;
        let repo_path = temp.path().join(".dotman");
        let config_path = temp.path().join("config.toml");

        // Create repo structure
        fs::create_dir_all(&repo_path)?;
        fs::create_dir_all(repo_path.join("commits"))?;
        fs::create_dir_all(repo_path.join("objects"))?;

        // Create empty index
        let index = Index::new();
        let index_path = repo_path.join("index.bin");
        index.save(&index_path)?;

        // Create HEAD file (required for repo initialization check)
        fs::write(repo_path.join("HEAD"), "")?;

        let mut config = Config::default();
        config.core.repo_path = repo_path.clone();
        config.save(&config_path)?;

        let ctx = DotmanContext {
            repo_path,
            config_path,
            config,
            no_pager: true,
        };

        Ok((temp, ctx))
    }

    #[test]
    fn test_execute_remove_tracked_file_cached() -> Result<()> {
        let (temp, ctx) = setup_test_context()?;

        let index_path = ctx.repo_path.join(INDEX_FILE);
        let mut index = Index::load(&index_path)?;

        let file_path = temp.path().join("test.txt");
        fs::write(&file_path, "test content")?;

        index.add_entry(FileEntry {
            path: file_path.clone(),
            hash: "test_hash".to_string(),
            size: 12,
            modified: 0,
            mode: 0o644,
            cached_hash: None,
        });
        index.save(&index_path)?;

        let result = execute(
            &ctx,
            &[file_path.to_string_lossy().to_string()],
            true,
            false,
            false,
            false,
        );
        assert!(result.is_ok());

        assert!(file_path.exists());
        let index = Index::load(&index_path)?;
        assert!(index.get_entry(&file_path).is_none());

        Ok(())
    }

    #[test]
    fn test_execute_remove_untracked_file_no_force() -> Result<()> {
        let (_temp, ctx) = setup_test_context()?;

        let result = execute(
            &ctx,
            &["untracked.txt".to_string()],
            false,
            false,
            false,
            false,
        );
        assert!(result.is_ok());

        Ok(())
    }

    #[test]
    fn test_execute_remove_untracked_file_with_force() -> Result<()> {
        let (temp, ctx) = setup_test_context()?;

        let file_path = temp.path().join("untracked.txt");
        fs::write(&file_path, "content")?;

        let result = execute(
            &ctx,
            &[file_path.to_string_lossy().to_string()],
            false,
            true,
            false,
            false,
        );
        assert!(result.is_ok());

        // File should still exist - rm should never delete actual files
        assert!(file_path.exists());

        Ok(())
    }

    #[test]
    fn test_execute_remove_multiple_files() -> Result<()> {
        let (temp, ctx) = setup_test_context()?;

        let index_path = ctx.repo_path.join(INDEX_FILE);
        let mut index = Index::load(&index_path)?;

        let file1 = temp.path().join("file1.txt");
        let file2 = temp.path().join("file2.txt");

        fs::write(&file1, "content1")?;
        fs::write(&file2, "content2")?;

        index.add_entry(FileEntry {
            path: file1.clone(),
            hash: "hash1".to_string(),
            size: 8,
            modified: 0,
            mode: 0o644,
            cached_hash: None,
        });

        index.add_entry(FileEntry {
            path: file2.clone(),
            hash: "hash2".to_string(),
            size: 8,
            modified: 0,
            mode: 0o644,
            cached_hash: None,
        });

        index.save(&index_path)?;

        let paths = vec![
            file1.to_string_lossy().to_string(),
            file2.to_string_lossy().to_string(),
        ];
        let result = execute(&ctx, &paths, true, false, false, false);
        assert!(result.is_ok());

        assert!(file1.exists());
        assert!(file2.exists());

        let index = Index::load(&index_path)?;
        assert!(index.get_entry(&file1).is_none());
        assert!(index.get_entry(&file2).is_none());

        Ok(())
    }

    #[test]
    fn test_execute_empty_paths() -> Result<()> {
        let (_temp, ctx) = setup_test_context()?;

        let result = execute(&ctx, &[], false, false, false, false);
        assert!(result.is_ok());

        Ok(())
    }

    #[test]
    fn test_execute_mixed_tracked_untracked() -> Result<()> {
        let (temp, ctx) = setup_test_context()?;

        // Add one file to index
        let index_path = ctx.repo_path.join(INDEX_FILE);
        let mut index = Index::load(&index_path)?;

        let tracked_file = temp.path().join("tracked.txt");
        fs::write(&tracked_file, "tracked content")?;

        index.add_entry(FileEntry {
            path: tracked_file.clone(),
            hash: "tracked_hash".to_string(),
            size: 15,
            modified: 0,
            mode: 0o644,
            cached_hash: None,
        });
        index.save(&index_path)?;

        // Try to remove both tracked and untracked
        let paths = vec![
            tracked_file.to_string_lossy().to_string(),
            "untracked.txt".to_string(),
        ];

        let result = execute(&ctx, &paths, true, false, false, false);
        assert!(result.is_ok());

        // Tracked file should be removed from index
        let index = Index::load(&index_path)?;
        assert!(index.get_entry(&tracked_file).is_none());

        Ok(())
    }

    #[test]
    fn test_execute_nonexistent_file() -> Result<()> {
        let (_temp, ctx) = setup_test_context()?;

        let result = execute(
            &ctx,
            &["/nonexistent/path/file.txt".to_string()],
            false,
            false,
            false,
            false,
        );
        assert!(result.is_ok());

        Ok(())
    }

    #[test]
    fn test_execute_with_absolute_and_relative_paths() -> Result<()> {
        let (temp, ctx) = setup_test_context()?;

        // Add files with different path types
        let index_path = ctx.repo_path.join(INDEX_FILE);
        let mut index = Index::load(&index_path)?;

        let abs_file = temp.path().join("abs.txt");
        let rel_file = PathBuf::from("rel.txt");

        fs::write(&abs_file, "absolute")?;

        index.add_entry(FileEntry {
            path: abs_file.clone(),
            hash: "abs_hash".to_string(),
            size: 8,
            modified: 0,
            mode: 0o644,
            cached_hash: None,
        });

        index.add_entry(FileEntry {
            path: rel_file.clone(),
            hash: "rel_hash".to_string(),
            size: 8,
            modified: 0,
            mode: 0o644,
            cached_hash: None,
        });

        index.save(&index_path)?;

        // Remove both
        let paths = vec![
            abs_file.to_string_lossy().to_string(),
            rel_file.to_string_lossy().to_string(),
        ];

        let result = execute(&ctx, &paths, true, false, false, false);
        assert!(result.is_ok());

        Ok(())
    }
}
