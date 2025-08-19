use crate::DotmanContext;
use crate::storage::snapshots::SnapshotManager;
use anyhow::Result;
use chrono::{Local, TimeZone};
use colored::Colorize;

pub fn execute(ctx: &DotmanContext, limit: usize, oneline: bool) -> Result<()> {
    ctx.ensure_repo_exists()?;

    let snapshot_manager =
        SnapshotManager::new(ctx.repo_path.clone(), ctx.config.core.compression_level);

    let snapshots = snapshot_manager.list_snapshots()?;

    if snapshots.is_empty() {
        super::print_info("No commits yet");
        return Ok(());
    }

    // Load and display commits
    let mut commits_displayed = 0;

    for snapshot_id in snapshots.iter().rev().take(limit) {
        let snapshot = snapshot_manager.load_snapshot(snapshot_id)?;
        let commit = &snapshot.commit;

        if oneline {
            // One-line format
            // Show last 8 chars for better uniqueness (timestamp is first 16 chars)
            let display_id = if commit.id.len() >= 8 {
                &commit.id[commit.id.len() - 8..]
            } else {
                &commit.id
            };
            println!("{} {}", display_id.yellow(), commit.message);
        } else {
            // Full format
            println!("{} {}", "commit".yellow(), commit.id);

            if let Some(parent) = &commit.parent {
                println!("{}: {}", "Parent".bold(), &parent[..8.min(parent.len())]);
            }

            println!("{}: {}", "Author".bold(), commit.author);

            let datetime = Local
                .timestamp_opt(commit.timestamp, 0)
                .single()
                .unwrap_or_else(Local::now);
            println!(
                "{}: {}",
                "Date".bold(),
                datetime.format("%Y-%m-%d %H:%M:%S")
            );

            println!("\n    {}\n", commit.message);
        }

        commits_displayed += 1;
    }

    if commits_displayed == 0 {
        super::print_info("No commits to display");
    } else if commits_displayed < snapshots.len() {
        println!(
            "\n{} (showing {} of {} commits)",
            "...".dimmed(),
            commits_displayed,
            snapshots.len()
        );
    }

    Ok(())
}
