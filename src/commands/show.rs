use crate::DotmanContext;
use crate::refs::resolver::RefResolver;
use crate::storage::snapshots::SnapshotManager;
use crate::utils::pager::{Pager, PagerConfig};
use anyhow::{Context, Result};
use chrono::{Local, TimeZone};
use colored::Colorize;

/// Execute show command - show various types of objects
///
/// # Errors
///
/// Returns an error if:
/// - The repository is not initialized
/// - The specified object cannot be resolved
/// - The commit does not exist
/// - Decompression fails
pub fn execute(ctx: &DotmanContext, object: &str) -> Result<()> {
    ctx.check_repo_initialized()?;

    // Use the reference resolver to handle HEAD, HEAD~n, branches, and short hashes
    let resolver = RefResolver::new(ctx.repo_path.clone());
    let commit_id = resolver
        .resolve(object)
        .with_context(|| format!("Failed to resolve reference: {object}"))?;

    let snapshot_manager =
        SnapshotManager::new(ctx.repo_path.clone(), ctx.config.core.compression_level);

    // Try to load as a commit
    let snapshot = snapshot_manager
        .load_snapshot(&commit_id)
        .with_context(|| format!("Failed to load object: {commit_id}"))?;

    let commit = &snapshot.commit;

    // Create pager with context
    let pager_config = PagerConfig::from_context(ctx, "show");
    let mut pager = Pager::builder().config(pager_config).build()?;
    let writer = pager.writer();

    // Display commit information
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
    writeln!(writer, "{}: {}", "Tree".bold(), &commit.tree_hash[..16])?;

    writeln!(writer, "\n    {}\n", commit.message)?;

    // Display file list
    writeln!(writer, "{}", "Files in this commit:".bold())?;

    let mut files: Vec<_> = snapshot.files.iter().collect();
    files.sort_by_key(|(path, _)| path.as_path());

    for (path, file) in files {
        writeln!(
            writer,
            "  {} {} {}",
            format!("{:06o}", file.mode).dimmed(),
            file.hash[..8.min(file.hash.len())].to_string().cyan(),
            path.display()
        )?;
    }

    writeln!(
        writer,
        "\n{}: {}",
        "Total files".bold(),
        snapshot.files.len()
    )?;

    pager.finish()?;

    Ok(())
}
