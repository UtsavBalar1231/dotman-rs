use crate::commands::context::CommandContext;
use crate::diff::binary::is_binary_file;
use crate::diff::unified::{
    UnifiedDiffConfig, generate_binary_diff_message, generate_unified_diff,
};
use crate::refs::resolver::RefResolver;
use crate::storage::FileStatus;
use crate::storage::index::Index;
use crate::storage::snapshots::{SnapshotFile, SnapshotManager};
use crate::utils::pager::{Pager, PagerConfig, PagerWriter};
use crate::{DotmanContext, INDEX_FILE};
use anyhow::{Context, Result};
use colored::Colorize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Compare two file collections and return their differences.
///
/// Compares files from two snapshots or file collections and returns a list of changes:
/// - Added: Files present in `to_files` but not in `from_files`
/// - Modified: Files present in both with different hashes
/// - Deleted: Files present in `from_files` but not in `to_files`
///
/// # Arguments
///
/// * `from_files` - The baseline file collection
/// * `to_files` - The target file collection to compare against
///
/// # Returns
///
/// A vector of [`FileStatus`] representing the differences between the two collections
fn compare_file_collections(
    from_files: &HashMap<PathBuf, SnapshotFile>,
    to_files: &HashMap<PathBuf, SnapshotFile>,
) -> Vec<FileStatus> {
    let mut statuses = Vec::new();

    // Find added and modified files
    for (path, to_file) in to_files {
        if let Some(from_file) = from_files.get(path) {
            // File exists in both - check if modified
            if from_file.hash != to_file.hash {
                statuses.push(FileStatus::Modified(path.clone()));
            }
        } else {
            // File only in "to" - it was added
            statuses.push(FileStatus::Added(path.clone()));
        }
    }

    // Find deleted files
    for path in from_files.keys() {
        if !to_files.contains_key(path) {
            statuses.push(FileStatus::Deleted(path.clone()));
        }
    }

    statuses
}

/// Generate unified diff for a single file.
///
/// Reads file contents and generates a unified diff, handling binary files appropriately.
///
/// # Arguments
///
/// * `writer` - Output writer for the diff
/// * `path` - File path for display
/// * `old_content` - Old file content (empty string for added files)
/// * `new_content` - New file content (empty string for deleted files)
/// * `ctx` - Dotman context with configuration
/// * `is_file_binary` - Whether the file is binary
fn generate_file_diff(
    writer: &mut dyn PagerWriter,
    path: &Path,
    old_content: &str,
    new_content: &str,
    ctx: &DotmanContext,
    is_file_binary: bool,
) -> Result<()> {
    if is_file_binary {
        generate_binary_diff_message(path, path, writer)?;
    } else {
        let algorithm = crate::diff::config_to_algorithm(&ctx.config.diff.algorithm);
        let config = UnifiedDiffConfig {
            context_lines: ctx.config.diff.context,
            algorithm,
            colorize: ctx.config.diff.color,
        };
        generate_unified_diff(old_content, new_content, path, path, &config, writer)?;
    }
    Ok(())
}

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
    let pager_config = PagerConfig::from_context(ctx, "diff");
    let mut pager = Pager::builder().config(pager_config).build()?;
    let writer = pager.writer();

    let index_path = ctx.repo_path.join(INDEX_FILE);
    let index = Index::load(&index_path)?;
    let home_dir = ctx.get_home_dir()?;
    let snapshot_manager =
        SnapshotManager::new(ctx.repo_path.clone(), ctx.config.core.compression_level);

    // Load HEAD snapshot to get committed files
    let ref_manager = crate::refs::RefManager::new(ctx.repo_path.clone());
    let committed_files = if let Some(commit_id) = ref_manager.get_head_commit()?
        && commit_id != "0".repeat(40)
    {
        snapshot_manager
            .load_snapshot(&commit_id)
            .ok()
            .map(|snapshot| snapshot.files)
    } else {
        None
    };

    let mut statuses = Vec::new();

    // Check staged files against working directory
    for (path, staged_entry) in &index.staged_entries {
        let abs_path = if path.is_relative() {
            home_dir.join(path)
        } else {
            path.clone()
        };

        if abs_path.exists() {
            // Hash file to check for modifications
            match crate::storage::file_ops::hash_file(&abs_path, staged_entry.cached_hash.as_ref())
            {
                Ok((current_hash, _)) => {
                    if current_hash != staged_entry.hash {
                        statuses.push(FileStatus::Modified(path.clone()));
                    }
                }
                Err(_) => {
                    // Hash failed - file might be binary or inaccessible, still show it
                    statuses.push(FileStatus::Modified(path.clone()));
                }
            }
        } else {
            statuses.push(FileStatus::Deleted(path.clone()));
        }
    }

    // Check committed files (not already staged) against working directory
    if let Some(ref files) = committed_files {
        for (path, snapshot_file) in files {
            // Skip if already staged (checked above)
            if index.staged_entries.contains_key(path) {
                continue;
            }

            let abs_path = if path.is_relative() {
                home_dir.join(path)
            } else {
                path.clone()
            };

            if abs_path.exists() {
                // Hash file to check for modifications (no cache available from snapshot)
                match crate::storage::file_ops::hash_file(&abs_path, None) {
                    Ok((current_hash, _)) => {
                        if current_hash != snapshot_file.hash {
                            statuses.push(FileStatus::Modified(path.clone()));
                        }
                    }
                    Err(_) => {
                        // Hash failed but file exists - show as modified
                        statuses.push(FileStatus::Modified(path.clone()));
                    }
                }
            } else {
                // File was deleted from disk
                statuses.push(FileStatus::Deleted(path.clone()));
            }
        }
    }

    if statuses.is_empty() {
        writeln!(writer, "No differences found")?;
        pager.finish()?;
        return Ok(());
    }

    // If unified diff is disabled, just show file status
    if !ctx.config.diff.unified {
        writeln!(
            writer,
            "{}",
            "Comparing working directory with index...".blue()
        )?;
        format_file_statuses(writer, &statuses)?;
        pager.finish()?;
        return Ok(());
    }

    // Generate unified diffs
    process_working_vs_index_diff(
        writer,
        &statuses,
        ctx,
        &index,
        committed_files.as_ref(),
        &snapshot_manager,
        &home_dir,
    )?;

    pager.finish()?;
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

    let pager_config = PagerConfig::from_context(ctx, "diff");
    let mut pager = Pager::builder().config(pager_config).build()?;
    let writer = pager.writer();

    let snapshot_manager =
        SnapshotManager::new(ctx.repo_path.clone(), ctx.config.core.compression_level);

    let snapshot = snapshot_manager
        .load_snapshot(&commit_id)
        .with_context(|| format!("Failed to load commit: {commit_id}"))?;

    let index_path = ctx.repo_path.join(INDEX_FILE);
    let index = Index::load(&index_path)?;

    // Convert staged entries to snapshot file format for comparison
    let mut working_files = HashMap::new();
    for (path, entry) in &index.staged_entries {
        working_files.insert(
            path.clone(),
            SnapshotFile {
                hash: entry.hash.clone(),
                mode: entry.mode,
                content_hash: entry.hash.clone(),
            },
        );
    }

    let statuses = compare_file_collections(&snapshot.files, &working_files);

    if statuses.is_empty() {
        writeln!(writer, "No differences found")?;
        pager.finish()?;
        return Ok(());
    }

    // If unified diff is disabled, just show file status
    if !ctx.config.diff.unified {
        writeln!(
            writer,
            "{}",
            format!(
                "Comparing commit {} with working directory...",
                commit_id[..8.min(commit_id.len())].yellow()
            )
            .blue()
        )?;
        format_file_statuses(writer, &statuses)?;
        pager.finish()?;
        return Ok(());
    }

    // Generate unified diffs
    let home_dir = ctx.get_home_dir()?;
    process_commit_vs_working_diff(
        writer,
        &statuses,
        ctx,
        &snapshot,
        &index,
        &snapshot_manager,
        &home_dir,
    )?;

    pager.finish()?;
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

    let pager_config = PagerConfig::from_context(ctx, "diff");
    let mut pager = Pager::builder().config(pager_config).build()?;
    let writer = pager.writer();

    let snapshot_manager =
        SnapshotManager::new(ctx.repo_path.clone(), ctx.config.core.compression_level);

    let from_snapshot = snapshot_manager
        .load_snapshot(&from_id)
        .with_context(|| format!("Failed to load commit: {from_id}"))?;
    let to_snapshot = snapshot_manager
        .load_snapshot(&to_id)
        .with_context(|| format!("Failed to load commit: {to_id}"))?;

    // Compare snapshots directly
    let statuses = compare_file_collections(&from_snapshot.files, &to_snapshot.files);

    if statuses.is_empty() {
        writeln!(writer, "No differences found")?;
        pager.finish()?;
        return Ok(());
    }

    // If unified diff is disabled, just show file status
    if !ctx.config.diff.unified {
        writeln!(
            writer,
            "{}",
            format!(
                "Comparing commit {} with commit {}...",
                from_id[..8.min(from_id.len())].yellow(),
                to_id[..8.min(to_id.len())].yellow()
            )
            .blue()
        )?;
        format_file_statuses(writer, &statuses)?;
        pager.finish()?;
        return Ok(());
    }

    // Generate unified diffs
    process_commits_diff(
        writer,
        &statuses,
        ctx,
        &from_snapshot,
        &to_snapshot,
        &snapshot_manager,
    )?;

    pager.finish()?;
    Ok(())
}

/// Format file status lists into grouped, colored output for the pager
///
/// Takes a slice of `FileStatus` items and groups them by status type (added, modified, deleted, untracked).
/// Outputs colored, formatted text showing file counts and paths with appropriate symbols:
/// - `+` for added/untracked files (green)
/// - `~` for modified files (yellow)
/// - `-` for deleted files (red)
///
/// Appends a summary line showing total counts for each category.
fn format_file_statuses(writer: &mut dyn PagerWriter, statuses: &[FileStatus]) -> Result<()> {
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
        writeln!(writer)?;
        writeln!(writer, "{}", "Added files:".green().bold())?;
        for path in &added {
            writeln!(writer, "  + {}", path.display())?;
        }
    }

    if !modified.is_empty() {
        writeln!(writer)?;
        writeln!(writer, "{}", "Modified files:".yellow().bold())?;
        for path in &modified {
            writeln!(writer, "  ~ {}", path.display())?;
        }
    }

    if !deleted.is_empty() {
        writeln!(writer)?;
        writeln!(writer, "{}", "Deleted files:".red().bold())?;
        for path in &deleted {
            writeln!(writer, "  - {}", path.display())?;
        }
    }

    writeln!(writer)?;
    writeln!(
        writer,
        "{}: {} added, {} modified, {} deleted",
        "Summary".bold(),
        added.len(),
        modified.len(),
        deleted.len()
    )?;

    Ok(())
}

/// Read content from object store by hash
fn read_object_content(snapshot_manager: &SnapshotManager, hash: &str) -> String {
    let bytes = snapshot_manager.read_object(hash).unwrap_or_default();
    String::from_utf8_lossy(&bytes).to_string()
}

/// Process and display diff for working vs index comparison
fn process_working_vs_index_diff(
    writer: &mut dyn PagerWriter,
    statuses: &[FileStatus],
    ctx: &DotmanContext,
    index: &Index,
    committed_files: Option<&HashMap<PathBuf, SnapshotFile>>,
    snapshot_manager: &SnapshotManager,
    home_dir: &Path,
) -> Result<()> {
    for status in statuses {
        match status {
            FileStatus::Modified(path) => {
                let full_path = if path.is_relative() {
                    home_dir.join(path)
                } else {
                    path.clone()
                };
                let new_content =
                    std::fs::read_to_string(&full_path).unwrap_or_else(|_| String::new());

                let old_content = get_old_content_for_working_diff(
                    path,
                    index,
                    committed_files,
                    snapshot_manager,
                );

                let is_binary = full_path.exists() && is_binary_file(&full_path).unwrap_or(false);
                generate_file_diff(writer, path, &old_content, &new_content, ctx, is_binary)?;
                writeln!(writer)?;
            }
            FileStatus::Added(path) => {
                let full_path = if path.is_relative() {
                    home_dir.join(path)
                } else {
                    path.clone()
                };
                let new_content =
                    std::fs::read_to_string(&full_path).unwrap_or_else(|_| String::new());
                let is_binary = full_path.exists() && is_binary_file(&full_path).unwrap_or(false);

                generate_file_diff(writer, path, "", &new_content, ctx, is_binary)?;
                writeln!(writer)?;
            }
            FileStatus::Deleted(path) => {
                let old_content = get_old_content_for_working_diff(
                    path,
                    index,
                    committed_files,
                    snapshot_manager,
                );

                generate_file_diff(writer, path, &old_content, "", ctx, false)?;
                writeln!(writer)?;
            }
            FileStatus::Untracked(_) => {}
        }
    }
    Ok(())
}

/// Get old content for working directory diff (from staged or committed)
fn get_old_content_for_working_diff(
    path: &Path,
    index: &Index,
    committed_files: Option<&HashMap<PathBuf, SnapshotFile>>,
    snapshot_manager: &SnapshotManager,
) -> String {
    index.staged_entries.get(path).map_or_else(
        || {
            committed_files
                .and_then(|files| {
                    files
                        .get(path)
                        .map(|sf| read_object_content(snapshot_manager, &sf.content_hash))
                })
                .unwrap_or_default()
        },
        |entry| read_object_content(snapshot_manager, &entry.hash),
    )
}

/// Process and display diff for commit vs working comparison
fn process_commit_vs_working_diff(
    writer: &mut dyn PagerWriter,
    statuses: &[FileStatus],
    ctx: &DotmanContext,
    snapshot: &crate::storage::snapshots::Snapshot,
    index: &Index,
    snapshot_manager: &SnapshotManager,
    home_dir: &Path,
) -> Result<()> {
    for status in statuses {
        match status {
            FileStatus::Modified(path) => {
                let old_content = snapshot.files.get(path).map_or_else(String::new, |file| {
                    read_object_content(snapshot_manager, &file.content_hash)
                });

                let new_content = index
                    .staged_entries
                    .get(path)
                    .map_or_else(String::new, |entry| {
                        read_object_content(snapshot_manager, &entry.hash)
                    });

                let full_path = home_dir.join(path);
                let is_binary = full_path.exists() && is_binary_file(&full_path).unwrap_or(false);

                generate_file_diff(writer, path, &old_content, &new_content, ctx, is_binary)?;
                writeln!(writer)?;
            }
            FileStatus::Added(path) => {
                let new_content = index
                    .staged_entries
                    .get(path)
                    .map_or_else(String::new, |entry| {
                        read_object_content(snapshot_manager, &entry.hash)
                    });

                let full_path = home_dir.join(path);
                let is_binary = full_path.exists() && is_binary_file(&full_path).unwrap_or(false);

                generate_file_diff(writer, path, "", &new_content, ctx, is_binary)?;
                writeln!(writer)?;
            }
            FileStatus::Deleted(path) => {
                let old_content = snapshot.files.get(path).map_or_else(String::new, |file| {
                    read_object_content(snapshot_manager, &file.content_hash)
                });

                generate_file_diff(writer, path, &old_content, "", ctx, false)?;
                writeln!(writer)?;
            }
            FileStatus::Untracked(_) => {}
        }
    }
    Ok(())
}

/// Process and display diff between two commits
fn process_commits_diff(
    writer: &mut dyn PagerWriter,
    statuses: &[FileStatus],
    ctx: &DotmanContext,
    from_snapshot: &crate::storage::snapshots::Snapshot,
    to_snapshot: &crate::storage::snapshots::Snapshot,
    snapshot_manager: &SnapshotManager,
) -> Result<()> {
    for status in statuses {
        match status {
            FileStatus::Modified(path) => {
                let old_content = from_snapshot
                    .files
                    .get(path)
                    .map_or_else(String::new, |file| {
                        read_object_content(snapshot_manager, &file.content_hash)
                    });

                let new_content = to_snapshot
                    .files
                    .get(path)
                    .map_or_else(String::new, |file| {
                        read_object_content(snapshot_manager, &file.content_hash)
                    });

                let is_binary = if !new_content.is_empty() {
                    new_content.contains('\0')
                } else if !old_content.is_empty() {
                    old_content.contains('\0')
                } else {
                    false
                };

                generate_file_diff(writer, path, &old_content, &new_content, ctx, is_binary)?;
                writeln!(writer)?;
            }
            FileStatus::Added(path) => {
                let new_content = to_snapshot
                    .files
                    .get(path)
                    .map_or_else(String::new, |file| {
                        read_object_content(snapshot_manager, &file.content_hash)
                    });

                let is_binary = new_content.contains('\0');

                generate_file_diff(writer, path, "", &new_content, ctx, is_binary)?;
                writeln!(writer)?;
            }
            FileStatus::Deleted(path) => {
                let old_content = from_snapshot
                    .files
                    .get(path)
                    .map_or_else(String::new, |file| {
                        read_object_content(snapshot_manager, &file.content_hash)
                    });

                let is_binary = old_content.contains('\0');

                generate_file_diff(writer, path, &old_content, "", ctx, is_binary)?;
                writeln!(writer)?;
            }
            FileStatus::Untracked(_) => {}
        }
    }
    Ok(())
}
