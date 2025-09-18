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
