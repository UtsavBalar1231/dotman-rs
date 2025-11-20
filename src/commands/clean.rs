use crate::commands::context::CommandContext;
use crate::output;
use crate::storage::index::Index;
use crate::{DotmanContext, INDEX_FILE};
use anyhow::{Context, Result};
use colored::Colorize;
use std::collections::HashSet;
use std::path::PathBuf;
use walkdir::WalkDir;

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

    // Find untracked files
    let untracked = find_untracked_files(ctx, &index)?;

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

/// Find all untracked files in the home directory
///
/// # Errors
///
/// Returns an error if failed to find home directory
fn find_untracked_files(ctx: &DotmanContext, index: &Index) -> Result<Vec<PathBuf>> {
    let home = dirs::home_dir().context("Could not find home directory")?;

    // Get tracked paths from HEAD snapshot (committed files)
    let snapshot_manager = ctx.create_snapshot_manager();
    let resolver = ctx.create_ref_resolver();

    let mut tracked_paths: HashSet<PathBuf> = if let Ok(commit_id) = resolver.resolve("HEAD") {
        let snapshot = snapshot_manager.load_snapshot(&commit_id)?;
        snapshot.files.keys().map(|p| home.join(p)).collect()
    } else {
        // No HEAD yet (empty repo)
        HashSet::new()
    };

    // Also include staged files as tracked
    for path in index.staged_entries.keys() {
        tracked_paths.insert(home.join(path));
    }

    let mut untracked = Vec::new();

    // Walk through home directory
    for entry in WalkDir::new(&home)
        .follow_links(ctx.config.tracking.follow_symlinks)
        .into_iter()
        .filter_entry(|e| {
            let path = e.path();
            // Skip hidden directories (except tracked ones)
            if path != home
                && path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with('.'))
            {
                return false;
            }
            // Skip the dotman repo itself
            if path == ctx.repo_path {
                return false;
            }
            true
        })
        .flatten()
    {
        let path = entry.path();
        if entry.file_type().is_file() && !tracked_paths.contains(path) {
            // Check against ignore patterns
            let relative_path = path.strip_prefix(&home).unwrap_or(path);
            if !crate::utils::should_ignore(relative_path, &ctx.config.tracking.ignore_patterns) {
                untracked.push(path.to_path_buf());
            }
        }
    }

    Ok(untracked)
}
