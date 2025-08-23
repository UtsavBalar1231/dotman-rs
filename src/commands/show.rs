use crate::DotmanContext;
use crate::refs::resolver::RefResolver;
use crate::storage::snapshots::SnapshotManager;
use anyhow::{Context, Result};
use chrono::{Local, TimeZone};
use colored::Colorize;

pub fn execute(ctx: &DotmanContext, object: &str) -> Result<()> {
    ctx.ensure_repo_exists()?;

    // Use the reference resolver to handle HEAD, HEAD~n, branches, and short hashes
    let resolver = RefResolver::new(ctx.repo_path.clone());
    let commit_id = resolver
        .resolve(object)
        .with_context(|| format!("Failed to resolve reference: {}", object))?;

    let snapshot_manager =
        SnapshotManager::new(ctx.repo_path.clone(), ctx.config.core.compression_level);

    // Try to load as a commit
    let snapshot = snapshot_manager
        .load_snapshot(&commit_id)
        .with_context(|| format!("Failed to load object: {}", commit_id))?;

    let commit = &snapshot.commit;

    // Display commit information
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
    println!("{}: {}", "Tree".bold(), &commit.tree_hash[..16]);

    println!("\n    {}\n", commit.message);

    // Display file list
    println!("{}", "Files in this commit:".bold());

    let mut files: Vec<_> = snapshot.files.iter().collect();
    files.sort_by_key(|(path, _)| path.as_path());

    for (path, file) in files {
        println!(
            "  {} {} {}",
            format!("{:06o}", file.mode).dimmed(),
            file.hash[..8].to_string().cyan(),
            path.display()
        );
    }

    println!("\n{}: {}", "Total files".bold(), snapshot.files.len());

    Ok(())
}
