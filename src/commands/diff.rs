use crate::storage::FileStatus;
use crate::storage::index::{Index, IndexDiffer};
use crate::storage::snapshots::SnapshotManager;
use crate::{DotmanContext, INDEX_FILE};
use anyhow::{Context, Result};
use colored::Colorize;

pub fn execute(ctx: &DotmanContext, from: Option<&str>, to: Option<&str>) -> Result<()> {
    ctx.ensure_repo_exists()?;

    match (from, to) {
        (None, None) => {
            // Diff working directory against index
            diff_working_vs_index(ctx)
        }
        (Some(commit), None) => {
            // Diff commit against working directory
            diff_commit_vs_working(ctx, commit)
        }
        (Some(from_commit), Some(to_commit)) => {
            // Diff between two commits
            diff_commits(ctx, from_commit, to_commit)
        }
        _ => anyhow::bail!("Invalid diff arguments"),
    }
}

fn diff_working_vs_index(ctx: &DotmanContext) -> Result<()> {
    use crate::commands::status::get_current_files;
    use crate::storage::index::ConcurrentIndex;

    super::print_info("Comparing working directory with index...");

    let index_path = ctx.repo_path.join(INDEX_FILE);
    let index = Index::load(&index_path)?;
    let concurrent_index = ConcurrentIndex::from_index(index);

    let current_files = get_current_files(ctx)?;
    let statuses = concurrent_index.get_status_parallel(&current_files);

    if statuses.is_empty() {
        println!("No differences found");
        return Ok(());
    }

    display_file_statuses(&statuses);

    Ok(())
}

fn diff_commit_vs_working(ctx: &DotmanContext, commit: &str) -> Result<()> {
    super::print_info(&format!(
        "Comparing commit {} with working directory...",
        commit[..8.min(commit.len())].yellow()
    ));

    let snapshot_manager =
        SnapshotManager::new(ctx.repo_path.clone(), ctx.config.core.compression_level);

    let snapshot = snapshot_manager
        .load_snapshot(commit)
        .with_context(|| format!("Failed to load commit: {}", commit))?;

    // Convert snapshot to index format for comparison
    let mut commit_index = Index::new();
    for (path, file) in &snapshot.files {
        commit_index.add_entry(crate::storage::FileEntry {
            path: path.clone(),
            hash: file.hash.clone(),
            size: 0,
            modified: snapshot.commit.timestamp,
            mode: file.mode,
        });
    }

    // Get current working directory state
    let index_path = ctx.repo_path.join(INDEX_FILE);
    let working_index = Index::load(&index_path)?;

    let statuses = IndexDiffer::diff(&commit_index, &working_index);

    if statuses.is_empty() {
        println!("No differences found");
        return Ok(());
    }

    display_file_statuses(&statuses);

    Ok(())
}

fn diff_commits(ctx: &DotmanContext, from: &str, to: &str) -> Result<()> {
    super::print_info(&format!(
        "Comparing commit {} with commit {}...",
        from[..8.min(from.len())].yellow(),
        to[..8.min(to.len())].yellow()
    ));

    let snapshot_manager =
        SnapshotManager::new(ctx.repo_path.clone(), ctx.config.core.compression_level);

    let from_snapshot = snapshot_manager
        .load_snapshot(from)
        .with_context(|| format!("Failed to load commit: {}", from))?;
    let to_snapshot = snapshot_manager
        .load_snapshot(to)
        .with_context(|| format!("Failed to load commit: {}", to))?;

    // Convert snapshots to index format
    let mut from_index = Index::new();
    for (path, file) in &from_snapshot.files {
        from_index.add_entry(crate::storage::FileEntry {
            path: path.clone(),
            hash: file.hash.clone(),
            size: 0,
            modified: from_snapshot.commit.timestamp,
            mode: file.mode,
        });
    }

    let mut to_index = Index::new();
    for (path, file) in &to_snapshot.files {
        to_index.add_entry(crate::storage::FileEntry {
            path: path.clone(),
            hash: file.hash.clone(),
            size: 0,
            modified: to_snapshot.commit.timestamp,
            mode: file.mode,
        });
    }

    let statuses = IndexDiffer::diff(&from_index, &to_index);

    if statuses.is_empty() {
        println!("No differences found");
        return Ok(());
    }

    display_file_statuses(&statuses);

    Ok(())
}

fn display_file_statuses(statuses: &[FileStatus]) {
    let mut added = Vec::new();
    let mut modified = Vec::new();
    let mut deleted = Vec::new();

    for status in statuses {
        match status {
            FileStatus::Added(p) => added.push(p),
            FileStatus::Modified(p) => modified.push(p),
            FileStatus::Deleted(p) => deleted.push(p),
            FileStatus::Untracked(p) => added.push(p), // Treat untracked as added in diff
        }
    }

    if !added.is_empty() {
        println!("\n{}", "Added files:".green().bold());
        for path in &added {
            println!("  + {}", path.display());
        }
    }

    if !modified.is_empty() {
        println!("\n{}", "Modified files:".yellow().bold());
        for path in &modified {
            println!("  ~ {}", path.display());
        }
    }

    if !deleted.is_empty() {
        println!("\n{}", "Deleted files:".red().bold());
        for path in &deleted {
            println!("  - {}", path.display());
        }
    }

    println!(
        "\n{}: {} added, {} modified, {} deleted",
        "Summary".bold(),
        added.len(),
        modified.len(),
        deleted.len()
    );
}
