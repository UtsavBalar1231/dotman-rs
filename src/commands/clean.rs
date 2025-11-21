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
    check_clean_flags(dry_run, force)?;

    let index_path = ctx.repo_path.join(INDEX_FILE);
    let index = Index::load(&index_path)?;
    let home = dirs::home_dir().context("Could not find home directory")?;

    let mut trie = DirTrie::new();
    let mut tracked_files = HashSet::new();

    get_committed_files(ctx, &home, &mut trie, &mut tracked_files)?;
    add_staged_files_to_tracking(&index, &home, &mut trie, &mut tracked_files);

    let untracked_files = find_untracked_files(&home, &ctx.repo_path, &trie, &tracked_files)?;
    let untracked =
        filter_ignored_files(untracked_files, &home, &ctx.config.tracking.ignore_patterns);

    if untracked.is_empty() {
        output::info("Already clean - no untracked files found");
        return Ok(());
    }

    display_clean_header(dry_run);
    let (removed_count, failed_count) = process_files_for_clean(&untracked, dry_run);
    print_clean_summary(dry_run, removed_count, failed_count);

    Ok(())
}

/// Check that clean command has appropriate safety flags
fn check_clean_flags(dry_run: bool, force: bool) -> Result<()> {
    if !dry_run && !force {
        output::error("clean requires either -n (dry run) or -f (force) flag for safety");
        output::info("Use 'dot clean -n' to see what would be removed");
        output::info("Use 'dot clean -f' to actually remove untracked files");
        return Err(anyhow::anyhow!("Missing required flag"));
    }
    Ok(())
}

/// Get committed files from HEAD snapshot
fn get_committed_files(
    ctx: &DotmanContext,
    home: &std::path::Path,
    trie: &mut DirTrie,
    tracked_files: &mut HashSet<PathBuf>,
) -> Result<()> {
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
                trie.insert_tracked_file(&abs_path, home);
                tracked_files.insert(abs_path);
            }
        }
    }
    Ok(())
}

/// Add staged files to tracking structures
fn add_staged_files_to_tracking(
    index: &Index,
    home: &std::path::Path,
    trie: &mut DirTrie,
    tracked_files: &mut HashSet<PathBuf>,
) {
    for path in index.staged_entries.keys() {
        let abs_path = if path.is_relative() {
            home.join(path)
        } else {
            path.clone()
        };
        trie.insert_tracked_file(&abs_path, home);
        tracked_files.insert(abs_path);
    }
}

/// Filter untracked files by ignore patterns
fn filter_ignored_files(
    untracked_files: Vec<PathBuf>,
    home: &std::path::Path,
    ignore_patterns: &[String],
) -> Vec<PathBuf> {
    untracked_files
        .into_iter()
        .filter(|file| {
            let relative_path = file.strip_prefix(home).unwrap_or(file);
            !crate::utils::should_ignore(relative_path, ignore_patterns)
        })
        .collect()
}

/// Display header for clean operation
fn display_clean_header(dry_run: bool) {
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
}

/// Process files for clean operation
fn process_files_for_clean(untracked: &[PathBuf], dry_run: bool) -> (usize, usize) {
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
    (removed_count, failed_count)
}

/// Print summary after clean operation
fn print_clean_summary(dry_run: bool, removed_count: usize, failed_count: usize) {
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
}
