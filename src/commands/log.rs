use crate::DotmanContext;
use crate::commands::context::CommandContext;
use crate::output;
use crate::refs::resolver::RefResolver;
use crate::storage::{Commit, snapshots::SnapshotManager};
use crate::utils::pager::{Pager, PagerConfig, PagerWriter};
use crate::utils::paths::expand_tilde;
use anyhow::Result;
use chrono::{Local, TimeZone};
use colored::Colorize;
use std::collections::HashSet;
use std::path::PathBuf;

/// Format and display a single commit
fn display_commit(writer: &mut dyn PagerWriter, commit: &Commit, oneline: bool) -> Result<()> {
    if oneline {
        let display_id = if commit.id.len() >= 8 {
            &commit.id[..8]
        } else {
            &commit.id
        };
        writeln!(writer, "{} {}", display_id.yellow(), commit.message)?;
    } else {
        writeln!(writer, "{} {}", "commit".yellow(), commit.id)?;

        if !commit.parents.is_empty() {
            let parent_display: Vec<String> = commit
                .parents
                .iter()
                .map(|p| p[..8.min(p.len())].to_string())
                .collect();
            writeln!(
                writer,
                "{}: {}",
                if commit.parents.len() > 1 {
                    "Parents"
                } else {
                    "Parent"
                }
                .bold(),
                parent_display.join(", ")
            )?;
        }

        writeln!(writer, "{}: {}", "Author".bold(), commit.author)?;

        let datetime = Local
            .timestamp_opt(commit.timestamp, 0)
            .single()
            .unwrap_or_else(Local::now);
        writeln!(
            writer,
            "{}: {}",
            "Date".bold(),
            datetime.format("%Y-%m-%d %H:%M:%S")
        )?;

        writeln!(writer, "\n    {}\n", commit.message)?;
    }
    Ok(())
}

/// Parse and normalize a single path argument
fn parse_path(ctx: &DotmanContext, path_str: &str) -> Result<PathBuf> {
    use std::path::Path;

    // Expand tilde
    let expanded = expand_tilde(Path::new(path_str))?;

    // Normalize to home-relative path (matches snapshot storage)
    let home_dir = ctx.get_home_dir()?;
    let normalized = expanded
        .strip_prefix(&home_dir)
        .map_or_else(|_| expanded.clone(), std::path::Path::to_path_buf);

    Ok(normalized)
}

/// Parse args into (`optional_ref`, paths)
/// Strategy: Try resolving first arg as ref. If succeeds, it's a ref and rest are paths.
/// If fails, all args are paths.
fn parse_log_args(
    ctx: &DotmanContext,
    args: &[String],
    resolver: &RefResolver,
) -> Result<(Option<String>, Vec<PathBuf>)> {
    if args.is_empty() {
        return Ok((None, vec![])); // Default: from HEAD, no filtering
    }

    // Try resolving first arg as reference
    let first_arg = &args[0];
    if let Ok(commit_id) = resolver.resolve(first_arg) {
        // First arg is a valid ref
        let paths = args[1..]
            .iter()
            .map(|p| parse_path(ctx, p))
            .collect::<Result<Vec<_>>>()?;
        Ok((Some(commit_id), paths))
    } else {
        // First arg is not a valid ref, treat all args as paths
        let paths = args
            .iter()
            .map(|p| parse_path(ctx, p))
            .collect::<Result<Vec<_>>>()?;
        Ok((None, paths))
    }
}

/// Check if commit modified any of the specified files
/// Returns true if:
/// - files list is empty (no filtering), OR
/// - commit modified at least one of the files (union behavior)
fn commit_touches_files(
    snapshot: &crate::storage::snapshots::Snapshot,
    prev_snapshot: Option<&crate::storage::snapshots::Snapshot>,
    filter_paths: &[PathBuf],
) -> bool {
    // No filtering - include all commits
    if filter_paths.is_empty() {
        return true;
    }

    // Check each filter path
    for path in filter_paths {
        // Check if file was added, modified, or deleted
        let current_hash = snapshot.files.get(path).map(|f| &f.hash);
        let prev_hash = prev_snapshot.and_then(|ps| ps.files.get(path).map(|f| &f.hash));

        // File changed if:
        // 1. Exists now but didn't before (added)
        // 2. Existed before but doesn't now (deleted)
        // 3. Hash changed (modified)
        if current_hash != prev_hash {
            return true; // At least one file changed - include commit
        }
    }

    false // None of the filter paths changed in this commit
}

/// Display commit history
///
/// # Errors
///
/// Returns an error if:
/// - The repository is not initialized
/// - The specified target reference cannot be resolved
/// - Failed to load snapshots
#[allow(clippy::too_many_lines)] // Detailed log formatting requires multiple sections
pub fn execute(
    ctx: &DotmanContext,
    args: &[String],
    limit: usize,
    oneline: bool,
    all: bool,
) -> Result<()> {
    ctx.check_repo_initialized()?;

    let snapshot_manager =
        SnapshotManager::new(ctx.repo_path.clone(), ctx.config.core.compression_level);

    let snapshots = snapshot_manager.list_snapshots()?;

    if snapshots.is_empty() {
        output::info("No commits yet");
        return Ok(());
    }

    // Create pager once at the start
    let pager_config = PagerConfig::from_context(ctx, "log");
    let mut pager = Pager::builder().config(pager_config).build()?;
    let writer = pager.writer();

    // Handle --all flag: show all commits including orphaned ones
    if all {
        let mut commits_displayed = 0;

        // Load all snapshots and sort by timestamp (most recent first)
        let mut snapshot_data: Vec<_> = snapshots
            .iter()
            .filter_map(|id| {
                snapshot_manager
                    .load_snapshot(id)
                    .ok()
                    .map(|snap| (id.clone(), snap))
            })
            .collect();

        snapshot_data.sort_by(|a, b| b.1.commit.timestamp.cmp(&a.1.commit.timestamp));

        let display_limit = limit.min(snapshot_data.len());

        for (_, snapshot) in snapshot_data.iter().take(display_limit) {
            display_commit(writer, &snapshot.commit, oneline)?;
            commits_displayed += 1;
        }

        if commits_displayed >= limit && snapshot_data.len() > limit {
            writeln!(
                writer,
                "\n{} (showing {} of {} total commits, use -n to see more)",
                "...".dimmed(),
                commits_displayed,
                snapshot_data.len()
            )?;
        }

        if commits_displayed > 0 {
            pager.finish()?;
        }

        return Ok(());
    }

    // Use the reference resolver to handle HEAD, HEAD~n, branches, and short hashes
    let resolver = RefResolver::new(ctx.repo_path.clone());

    // Parse arguments into ref and paths
    let (start_commit, filter_paths) = parse_log_args(ctx, args, &resolver)?;

    let mut commits_displayed = 0;

    // Determine starting commit
    let starting_commit_id = if let Some(commit_id) = start_commit {
        commit_id
    } else {
        // Default to HEAD if it exists
        if let Ok(id) = resolver.resolve("HEAD") {
            id
        } else {
            output::info("No commits yet");
            return Ok(());
        }
    };

    // Follow parent chain from starting commit
    let mut current_commit_id = Some(starting_commit_id);
    let mut visited = HashSet::new();

    while let Some(ref commit_id) = current_commit_id {
        if commits_displayed >= limit {
            break;
        }

        // Prevent infinite loops
        if visited.contains(commit_id) {
            break;
        }
        visited.insert(commit_id.clone());

        let Ok(snapshot) = snapshot_manager.load_snapshot(commit_id) else {
            break;
        };

        // Load parent snapshot for comparison (to detect changes in this commit)
        let parent_snapshot = snapshot
            .commit
            .parents
            .first()
            .and_then(|pid| snapshot_manager.load_snapshot(pid).ok());

        // Apply file filtering (compare current commit vs its parent)
        if commit_touches_files(&snapshot, parent_snapshot.as_ref(), &filter_paths) {
            display_commit(writer, &snapshot.commit, oneline)?;
            commits_displayed += 1;
        }

        // Move to first parent
        current_commit_id = snapshot.commit.parents.first().cloned();
    }

    if commits_displayed == 0 {
        if filter_paths.is_empty() {
            output::info("No commits yet");
        } else {
            let path_list = filter_paths
                .iter()
                .map(|p| format!("'{}'", p.display()))
                .collect::<Vec<_>>()
                .join(", ");
            output::info(&format!("No commits found that modified {path_list}"));
        }
    } else if commits_displayed >= limit {
        // Only show truncation indicator if we hit the display limit
        writeln!(
            writer,
            "\n{} (showing {} commits, use -n to see more)",
            "...".dimmed(),
            commits_displayed
        )?;
    }

    if commits_displayed > 0 {
        pager.finish()?;
    }

    Ok(())
}
