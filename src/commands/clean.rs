use crate::commands::context::CommandContext;
use crate::output;
use crate::scanner::{DirTrie, find_untracked_files};
use crate::storage::index::Index;
use crate::{DotmanContext, INDEX_FILE};
use anyhow::{Context, Result};
use colored::Colorize;
use std::collections::HashSet;
use std::path::PathBuf;

/// Execute clean command to remove untracked files
///
/// # Errors
///
/// Returns an error if:
/// - Repository is not initialized
/// - Failed to load index
/// - Failed to find untracked files
pub fn execute(ctx: &DotmanContext, dry_run: bool, force: bool) -> Result<()> {
    ctx.ensure_initialized()?;

    // Safety check: require either -n or -f flag
    if !dry_run && !force {
        output::error("clean requires either -n (dry run) or -f (force) flag for safety");
        output::info("Use 'dot clean -n' to see what would be removed");
        output::info("Use 'dot clean -f' to actually remove untracked files");
        return Ok(());
    }

    // Load index to get tracked files
    let index_path = ctx.repo_path.join(INDEX_FILE);
    let index = Index::load(&index_path)?;

    let home = dirs::home_dir().context("Could not find home directory")?;

    // Build trie and tracked files set for untracked file discovery
    let mut trie = DirTrie::new();
    let mut tracked_files = HashSet::new();

    // Get committed files from HEAD snapshot
    let ref_manager = crate::refs::RefManager::new(ctx.repo_path.clone());
    if let Some(commit_id) = ref_manager.get_head_commit()?
        && commit_id != "0".repeat(40)
    {
        let snapshot_manager = crate::storage::snapshots::SnapshotManager::new(
            ctx.repo_path.clone(),
            ctx.config.core.compression_level,
        );
        if let Ok(snapshot) = snapshot_manager.load_snapshot(&commit_id) {
            for path in snapshot.files.keys() {
                let abs_path = if path.is_relative() {
                    home.join(path)
                } else {
                    path.clone()
                };
                trie.insert_tracked_file(&abs_path, &home);
                tracked_files.insert(abs_path);
            }
        }
    }

    // Also include staged files
    for path in index.staged_entries.keys() {
        let abs_path = if path.is_relative() {
            home.join(path)
        } else {
            path.clone()
        };
        trie.insert_tracked_file(&abs_path, &home);
        tracked_files.insert(abs_path);
    }

    // Find untracked files using shared scanner
    let untracked_files = find_untracked_files(&home, &ctx.repo_path, &trie, &tracked_files)?;

    // Filter by ignore patterns
    let untracked: Vec<PathBuf> = untracked_files
        .into_iter()
        .filter(|file| {
            let relative_path = file.strip_prefix(&home).unwrap_or(file);
            !crate::utils::should_ignore(relative_path, &ctx.config.tracking.ignore_patterns)
        })
        .collect();

    if untracked.is_empty() {
        output::info("Already clean - no untracked files found");
        return Ok(());
    }

    // Display what will be/was removed
    if dry_run {
        println!(
            "\n{}",
            "Would remove the following untracked files:"
                .yellow()
                .bold()
        );
    } else {
        println!("\n{}", "Removing untracked files:".red().bold());
    }

    let mut removed_count = 0;
    let mut failed_count = 0;

    let total_files = untracked.len();
    let mut progress = output::start_progress(
        if dry_run {
            "Checking files"
        } else {
            "Removing files"
        },
        total_files,
    );

    for (i, path) in untracked.iter().enumerate() {
        if dry_run {
            println!("  {} {}", "would remove:".yellow(), path.display());
            removed_count += 1;
        } else {
            // Actually remove the file
            match std::fs::remove_file(path) {
                Ok(()) => {
                    println!("  {} {}", "removed:".red(), path.display());
                    removed_count += 1;
                }
                Err(e) => {
                    output::warning(&format!("Failed to remove {}: {e}", path.display()));
                    failed_count += 1;
                }
            }
        }
        progress.update(i + 1);
    }

    progress.finish();

    // Print summary
    println!();
    if dry_run {
        output::info(&format!(
            "{removed_count} untracked file(s) would be removed"
        ));
        output::info("Run 'dot clean -f' to actually remove these files");
    } else {
        output::success(&format!("Removed {removed_count} untracked file(s)"));
        if failed_count > 0 {
            output::warning(&format!("Failed to remove {failed_count} file(s)"));
        }
    }

    Ok(())
}
