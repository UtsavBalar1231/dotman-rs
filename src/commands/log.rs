use crate::DotmanContext;
use crate::output;
use crate::refs::resolver::RefResolver;
use crate::storage::{Commit, snapshots::SnapshotManager};
use crate::utils::pager::{Pager, PagerConfig, PagerWriter};
use anyhow::Result;
use chrono::{Local, TimeZone};
use colored::Colorize;
use std::collections::HashSet;

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

        if let Some(parent) = &commit.parent {
            writeln!(
                writer,
                "{}: {}",
                "Parent".bold(),
                &parent[..8.min(parent.len())]
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
    target: Option<&str>,
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

    let mut commits_displayed = 0;

    // Use the reference resolver to handle HEAD, HEAD~n, branches, and short hashes
    let resolver = RefResolver::new(ctx.repo_path.clone());

    // If a target is specified, start from that commit and follow parent chain
    if let Some(target_ref) = target {
        let start_commit_id = resolver.resolve(target_ref)?;

        // Follow parent chain from the starting commit
        let mut current_commit_id = Some(start_commit_id);
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
                break; // Stop if we can't find the commit
            };

            let commit = &snapshot.commit;
            display_commit(writer, commit, oneline)?;
            commits_displayed += 1;

            // Move to parent commit
            current_commit_id.clone_from(&commit.parent);
        }
    } else {
        // Try to get HEAD commit, if it exists
        let head_result = resolver.resolve("HEAD");

        if let Ok(head_commit_id) = head_result {
            // Follow parent chain from HEAD
            let mut current_commit_id = Some(head_commit_id);
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

                let commit = &snapshot.commit;
                display_commit(writer, commit, oneline)?;
                commits_displayed += 1;

                // Move to parent commit
                current_commit_id.clone_from(&commit.parent);
            }
        }
    }

    if commits_displayed == 0 {
        output::info("No commits to display");
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
