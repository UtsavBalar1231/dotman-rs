use crate::refs::resolver::RefResolver;
use crate::storage::FileStatus;
use crate::storage::index::{Index, IndexDiffer};
use crate::storage::snapshots::SnapshotManager;
use crate::utils::pager::PagerOutput;
use crate::{DotmanContext, INDEX_FILE};
use anyhow::{Context, Result};
use colored::Colorize;

/// Execute diff command to show differences between commits or working directory
///
/// # Errors
///
/// Returns an error if:
/// - Repository is not initialized
/// - Failed to resolve commit references
/// - Failed to load snapshots or index
pub fn execute(ctx: &DotmanContext, from: Option<&str>, to: Option<&str>) -> Result<()> {
    ctx.check_repo_initialized()?;

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
        _ => Err(anyhow::anyhow!("Invalid diff arguments")),
    }
}

/// Compare working directory against the index
///
/// # Errors
///
/// Returns an error if failed to load index or get file status
fn diff_working_vs_index(ctx: &DotmanContext) -> Result<()> {
    use crate::commands::status::get_current_files;

    let mut output = PagerOutput::new(ctx, ctx.no_pager);
    output.appendln(&format!(
        "{}",
        "Comparing working directory with index...".blue()
    ));

    let index_path = ctx.repo_path.join(INDEX_FILE);
    let index = Index::load(&index_path)?;

    let current_files = get_current_files(ctx)?;
    let statuses = index.get_status_parallel(&current_files);

    if statuses.is_empty() {
        output.appendln("No differences found");
        output.show()?;
        return Ok(());
    }

    format_file_statuses(&mut output, &statuses);
    output.show()?;

    Ok(())
}

/// Compare a commit against the working directory
///
/// # Errors
///
/// Returns an error if:
/// - Failed to resolve commit reference
/// - Failed to load snapshot or index
fn diff_commit_vs_working(ctx: &DotmanContext, commit: &str) -> Result<()> {
    // Resolve the commit reference
    let resolver = RefResolver::new(ctx.repo_path.clone());
    let commit_id = resolver
        .resolve(commit)
        .with_context(|| format!("Failed to resolve reference: {commit}"))?;

    let mut output = PagerOutput::new(ctx, ctx.no_pager);
    output.appendln(&format!(
        "{}",
        format!(
            "Comparing commit {} with working directory...",
            commit_id[..8.min(commit_id.len())].yellow()
        )
        .blue()
    ));

    let snapshot_manager =
        SnapshotManager::new(ctx.repo_path.clone(), ctx.config.core.compression_level);

    let snapshot = snapshot_manager
        .load_snapshot(&commit_id)
        .with_context(|| format!("Failed to load commit: {commit_id}"))?;

    // Convert snapshot to index format for comparison
    let mut commit_index = Index::new();
    for (path, file) in &snapshot.files {
        commit_index.add_entry(crate::storage::FileEntry {
            path: path.clone(),
            hash: file.hash.clone(),
            size: 0,
            modified: snapshot.commit.timestamp,
            mode: file.mode,
            cached_hash: None,
        });
    }

    let index_path = ctx.repo_path.join(INDEX_FILE);
    let working_index = Index::load(&index_path)?;

    let statuses = IndexDiffer::diff(&commit_index, &working_index);

    if statuses.is_empty() {
        output.appendln("No differences found");
        output.show()?;
        return Ok(());
    }

    format_file_statuses(&mut output, &statuses);
    output.show()?;

    Ok(())
}

/// Compare two commits
///
/// # Errors
///
/// Returns an error if:
/// - Failed to resolve commit references
/// - Failed to load snapshots
fn diff_commits(ctx: &DotmanContext, from: &str, to: &str) -> Result<()> {
    // Resolve the commit references
    let resolver = RefResolver::new(ctx.repo_path.clone());
    let from_id = resolver
        .resolve(from)
        .with_context(|| format!("Failed to resolve reference: {from}"))?;
    let to_id = resolver
        .resolve(to)
        .with_context(|| format!("Failed to resolve reference: {to}"))?;

    let mut output = PagerOutput::new(ctx, ctx.no_pager);
    output.appendln(&format!(
        "{}",
        format!(
            "Comparing commit {} with commit {}...",
            from_id[..8.min(from_id.len())].yellow(),
            to_id[..8.min(to_id.len())].yellow()
        )
        .blue()
    ));

    let snapshot_manager =
        SnapshotManager::new(ctx.repo_path.clone(), ctx.config.core.compression_level);

    let from_snapshot = snapshot_manager
        .load_snapshot(&from_id)
        .with_context(|| format!("Failed to load commit: {from_id}"))?;
    let to_snapshot = snapshot_manager
        .load_snapshot(&to_id)
        .with_context(|| format!("Failed to load commit: {to_id}"))?;

    // Convert snapshots to index format
    let mut from_index = Index::new();
    for (path, file) in &from_snapshot.files {
        from_index.add_entry(crate::storage::FileEntry {
            path: path.clone(),
            hash: file.hash.clone(),
            size: 0,
            modified: from_snapshot.commit.timestamp,
            mode: file.mode,
            cached_hash: None,
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
            cached_hash: None,
        });
    }

    let statuses = IndexDiffer::diff(&from_index, &to_index);

    if statuses.is_empty() {
        output.appendln("No differences found");
        output.show()?;
        return Ok(());
    }

    format_file_statuses(&mut output, &statuses);
    output.show()?;

    Ok(())
}

fn format_file_statuses(output: &mut PagerOutput, statuses: &[FileStatus]) {
    let mut added = Vec::new();
    let mut modified = Vec::new();
    let mut deleted = Vec::new();

    for status in statuses {
        match status {
            FileStatus::Added(p) | FileStatus::Untracked(p) => added.push(p),
            FileStatus::Modified(p) => modified.push(p),
            FileStatus::Deleted(p) => deleted.push(p),
        }
    }

    if !added.is_empty() {
        output.appendln("");
        output.appendln(&format!("{}", "Added files:".green().bold()));
        for path in &added {
            output.appendln(&format!("  + {}", path.display()));
        }
    }

    if !modified.is_empty() {
        output.appendln("");
        output.appendln(&format!("{}", "Modified files:".yellow().bold()));
        for path in &modified {
            output.appendln(&format!("  ~ {}", path.display()));
        }
    }

    if !deleted.is_empty() {
        output.appendln("");
        output.appendln(&format!("{}", "Deleted files:".red().bold()));
        for path in &deleted {
            output.appendln(&format!("  - {}", path.display()));
        }
    }

    output.appendln("");
    output.appendln(&format!(
        "{}: {} added, {} modified, {} deleted",
        "Summary".bold(),
        added.len(),
        modified.len(),
        deleted.len()
    ));
}
