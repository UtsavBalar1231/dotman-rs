use crate::DotmanContext;
use crate::refs::resolver::RefResolver;
use crate::storage::{Commit, snapshots::SnapshotManager};
use crate::utils::pager::PagerOutput;
use anyhow::Result;
use chrono::{Local, TimeZone};
use colored::Colorize;
use std::collections::HashSet;

/// Format and display a single commit
fn display_commit(output: &mut PagerOutput, commit: &Commit, oneline: bool) {
    if oneline {
        let display_id = if commit.id.len() >= 8 {
            &commit.id[..8]
        } else {
            &commit.id
        };
        output.appendln(&format!("{} {}", display_id.yellow(), commit.message));
    } else {
        output.appendln(&format!("{} {}", "commit".yellow(), commit.id));

        if let Some(parent) = &commit.parent {
            output.appendln(&format!(
                "{}: {}",
                "Parent".bold(),
                &parent[..8.min(parent.len())]
            ));
        }

        output.appendln(&format!("{}: {}", "Author".bold(), commit.author));

        let datetime = Local
            .timestamp_opt(commit.timestamp, 0)
            .single()
            .unwrap_or_else(Local::now);
        output.appendln(&format!(
            "{}: {}",
            "Date".bold(),
            datetime.format("%Y-%m-%d %H:%M:%S")
        ));

        output.appendln(&format!("\n    {}\n", commit.message));
    }
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
) -> Result<()> {
    ctx.check_repo_initialized()?;

    let snapshot_manager =
        SnapshotManager::new(ctx.repo_path.clone(), ctx.config.core.compression_level);

    let snapshots = snapshot_manager.list_snapshots()?;

    if snapshots.is_empty() {
        super::print_info("No commits yet");
        return Ok(());
    }

    let mut output = PagerOutput::new(ctx, ctx.no_pager);

    let mut commits_displayed = 0;

    // Use the reference resolver to handle HEAD, HEAD~n, branches, and short hashes
    let resolver = RefResolver::new(ctx.repo_path.clone());

    // If a target is specified, start from that commit and follow parent chain
    if let Some(target_ref) = target {
        let start_commit_id = resolver.resolve(target_ref)?;

        // Follow parent chain from the starting commit
        let mut current_commit_id = Some(start_commit_id);
        let mut visited = HashSet::new();

        while let Some(commit_id) = current_commit_id {
            if commits_displayed >= limit {
                break;
            }

            // Prevent infinite loops
            if visited.contains(&commit_id) {
                break;
            }
            visited.insert(commit_id.clone());

            let Ok(snapshot) = snapshot_manager.load_snapshot(&commit_id) else {
                break; // Stop if we can't find the commit
            };

            let commit = &snapshot.commit;
            display_commit(&mut output, commit, oneline);
            commits_displayed += 1;

            // Move to parent commit
            #[allow(clippy::assigning_clones)]
            {
                current_commit_id = commit.parent.clone();
            }
        }
    } else {
        // Try to get HEAD commit, if it exists
        let head_result = resolver.resolve("HEAD");

        if let Ok(head_commit_id) = head_result {
            // Follow parent chain from HEAD
            let mut current_commit_id = Some(head_commit_id);
            let mut visited = HashSet::new();

            while let Some(commit_id) = current_commit_id {
                if commits_displayed >= limit {
                    break;
                }

                // Prevent infinite loops
                if visited.contains(&commit_id) {
                    break;
                }
                visited.insert(commit_id.clone());

                let Ok(snapshot) = snapshot_manager.load_snapshot(&commit_id) else {
                    break;
                };

                let commit = &snapshot.commit;
                display_commit(&mut output, commit, oneline);
                commits_displayed += 1;

                // Move to parent commit
                #[allow(clippy::assigning_clones)]
                {
                    current_commit_id = commit.parent.clone();
                }
            }
        }
    }

    if commits_displayed == 0 {
        super::print_info("No commits to display");
    } else if commits_displayed < snapshots.len() {
        output.appendln(&format!(
            "\n{} (showing {} of {} commits)",
            "...".dimmed(),
            commits_displayed,
            snapshots.len()
        ));
    }

    if commits_displayed > 0 {
        output.show()?;
    }

    Ok(())
}
