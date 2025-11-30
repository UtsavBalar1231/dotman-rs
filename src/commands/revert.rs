use crate::output;
use crate::refs::resolver::RefResolver;
use crate::storage::file_ops::hash_bytes;
use crate::storage::index::Index;
use crate::storage::snapshots::{SnapshotFile, SnapshotManager};
use crate::storage::{Commit, FileEntry, FileStatus};
use crate::utils::{commit::generate_commit_id, get_precise_timestamp, get_user_from_config};
use crate::{DotmanContext, INDEX_FILE};
use anyhow::{Context, Result};
use colored::Colorize;
use std::collections::HashMap;
use std::fmt::Write as FmtWrite;
use std::fs;
use std::path::PathBuf;

/// Compare two file collections and return their differences.
///
/// Compares files from two snapshots and returns a list of changes:
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

/// Execute revert command - revert changes from a specific commit
///
/// # Errors
///
/// Returns an error if:
/// - The repository is not initialized
/// - The working directory has uncommitted changes (unless --force is used)
/// - The specified commit cannot be resolved
/// - The revert operation creates conflicts
/// - Commit creation fails
pub fn execute(ctx: &DotmanContext, commit_ref: &str, _no_edit: bool, force: bool) -> Result<()> {
    ctx.check_repo_initialized()?;

    if !force {
        let status_output = check_working_directory_clean(ctx)?;
        if !status_output {
            return Err(anyhow::anyhow!(
                "You have uncommitted changes. Use --force to override or commit your changes first."
            ));
        }
    }

    // Resolve the commit reference
    let resolver = RefResolver::new(ctx.repo_path.clone());
    let target_commit_id = resolver
        .resolve(commit_ref)
        .with_context(|| format!("Failed to resolve commit reference: {commit_ref}"))?;

    let snapshot_manager = SnapshotManager::with_permissions(
        ctx.repo_path.clone(),
        ctx.config.core.compression_level,
        ctx.config.tracking.preserve_permissions,
    );

    let target_snapshot = snapshot_manager
        .load_snapshot(&target_commit_id)
        .with_context(|| format!("Failed to load commit: {target_commit_id}"))?;

    let display_target = if target_commit_id.len() >= 8 {
        &target_commit_id[..8]
    } else {
        &target_commit_id
    };

    output::info(&format!(
        "Reverting commit {} \"{}\"",
        display_target.yellow(),
        target_snapshot.commit.message
    ));

    // Calculate what changes need to be reverted
    let changes_to_revert = calculate_revert_changes(ctx, &target_snapshot, &snapshot_manager)?;

    if changes_to_revert.is_empty() {
        output::info("No changes to revert.");
        return Ok(());
    }

    // Show what will be reverted
    display_revert_summary(&changes_to_revert);

    // Apply the inverse changes to the working directory and index
    apply_revert_changes(ctx, &changes_to_revert, &snapshot_manager)?;

    let revert_message = format!("Revert \"{}\"", target_snapshot.commit.message);
    create_revert_commit(ctx, &revert_message)?;

    output::success(&format!(
        "Reverted commit {} - \"{}\"",
        display_target.yellow(),
        target_snapshot.commit.message
    ));

    Ok(())
}

/// Check if the working directory is clean (no uncommitted changes)
///
/// # Arguments
///
/// * `ctx` - The dotman context containing repository configuration
///
/// # Returns
///
/// Returns `Ok(true)` if the working directory is clean, `Ok(false)` otherwise
///
/// # Errors
///
/// Returns an error if:
/// - The index file cannot be loaded
/// - Current files cannot be retrieved
fn check_working_directory_clean(ctx: &DotmanContext) -> Result<bool> {
    use crate::commands::status::get_current_files;

    let index_path = ctx.repo_path.join(INDEX_FILE);
    let index = Index::load(&index_path)?;

    let current_files = get_current_files(ctx)?;
    let statuses = index.get_status_parallel(&current_files);

    Ok(statuses.is_empty())
}

/// Calculate the changes needed to revert a commit
///
/// Compares the target commit with its parent to determine what changes
/// the original commit made, then generates inverse operations.
///
/// # Arguments
///
/// * `_ctx` - The dotman context (currently unused but kept for consistency)
/// * `target_snapshot` - The snapshot of the commit being reverted
/// * `snapshot_manager` - Manager for loading commit snapshots
///
/// # Returns
///
/// Returns a vector of `RevertChange` operations needed to undo the commit
///
/// # Errors
///
/// Returns an error if:
/// - The parent commit cannot be loaded
/// - Snapshot comparison fails
fn calculate_revert_changes(
    _ctx: &DotmanContext,
    target_snapshot: &crate::storage::snapshots::Snapshot,
    snapshot_manager: &SnapshotManager,
) -> Result<Vec<RevertChange>> {
    let mut revert_changes = Vec::new();

    if let Some(parent_id) = target_snapshot.commit.parents.first() {
        // Commit has a parent - compare with parent to see what the original commit did
        let parent_snapshot = snapshot_manager
            .load_snapshot(parent_id)
            .with_context(|| format!("Failed to load parent commit: {parent_id}"))?;

        // Find what changed from parent to target
        let changes = compare_file_collections(&parent_snapshot.files, &target_snapshot.files);

        // Create inverse operations
        for change in changes {
            match change {
                FileStatus::Added(path) => {
                    // Original commit added this file - revert by deleting it
                    revert_changes.push(RevertChange::Delete(path));
                }
                FileStatus::Modified(path) => {
                    // Original commit modified this file - revert by restoring parent version
                    if let Some(parent_file) = parent_snapshot.files.get(&path) {
                        revert_changes.push(RevertChange::Restore {
                            path: path.clone(),
                            content_hash: parent_file.content_hash.clone(),
                            mode: parent_file.mode,
                        });
                    }
                }
                FileStatus::Deleted(path) => {
                    // Original commit deleted this file - revert by restoring it from parent
                    if let Some(parent_file) = parent_snapshot.files.get(&path) {
                        revert_changes.push(RevertChange::Restore {
                            path: path.clone(),
                            content_hash: parent_file.content_hash.clone(),
                            mode: parent_file.mode,
                        });
                    }
                }
                FileStatus::Untracked(_) => {
                    // This shouldn't happen in commit comparison
                }
            }
        }
    } else {
        // This is a root commit (no parent) - revert by deleting all files it added
        for path in target_snapshot.files.keys() {
            revert_changes.push(RevertChange::Delete(path.clone()));
        }
    }

    Ok(revert_changes)
}

/// Display a summary of changes that will be reverted
///
/// Shows each file that will be deleted or restored, along with
/// a count of total operations.
///
/// # Arguments
///
/// * `changes` - Slice of revert changes to display
fn display_revert_summary(changes: &[RevertChange]) {
    println!();
    output::info("Changes to be reverted:");

    let mut deletions = 0;
    let mut restorations = 0;

    for change in changes {
        match change {
            RevertChange::Delete(path) => {
                println!("  {} {}", "-".red().bold(), path.display());
                deletions += 1;
            }
            RevertChange::Restore { path, .. } => {
                println!("  {} {}", "+".green().bold(), path.display());
                restorations += 1;
            }
        }
    }

    println!();
    println!(
        "{}: {} restorations, {} deletions",
        "Summary".bold(),
        restorations,
        deletions
    );
    println!();
}

/// Apply revert changes to the working directory and index
///
/// Executes the calculated revert operations by deleting or restoring files
/// as needed, then updates the index to reflect the changes.
///
/// # Arguments
///
/// * `ctx` - The dotman context containing repository configuration
/// * `changes` - Slice of revert changes to apply
/// * `snapshot_manager` - Manager for restoring file content from objects
///
/// # Errors
///
/// Returns an error if:
/// - Home directory cannot be found
/// - Index file cannot be loaded or saved
/// - File deletion fails
/// - File restoration fails
/// - Directory creation fails
/// - File permissions cannot be set
fn apply_revert_changes(
    ctx: &DotmanContext,
    changes: &[RevertChange],
    snapshot_manager: &SnapshotManager,
) -> Result<()> {
    let home = dirs::home_dir().context("Could not find home directory")?;

    // Load current index
    let index_path = ctx.repo_path.join(INDEX_FILE);
    let mut index = Index::load(&index_path)?;

    for change in changes {
        match change {
            RevertChange::Delete(path) => {
                // Delete file from working directory
                let abs_path = if path.is_absolute() {
                    path.clone()
                } else {
                    home.join(path)
                };

                if abs_path.exists() {
                    fs::remove_file(&abs_path).with_context(|| {
                        format!("Failed to delete file: {}", abs_path.display())
                    })?;
                }

                // Remove from staged entries
                index.staged_entries.remove(path);
            }
            RevertChange::Restore {
                path,
                content_hash,
                mode,
            } => {
                // Restore file content to working directory
                let abs_path = if path.is_absolute() {
                    path.clone()
                } else {
                    home.join(path)
                };

                // Create parent directories if needed
                if let Some(parent) = abs_path.parent() {
                    fs::create_dir_all(parent)?;
                }

                // Restore file content from object store
                snapshot_manager
                    .restore_file_content(content_hash, &abs_path)
                    .with_context(|| {
                        format!("Failed to restore file content: {}", abs_path.display())
                    })?;

                // Set file permissions using cross-platform module
                let permissions = crate::utils::permissions::FilePermissions::from_mode(*mode);
                permissions.apply_to_path(
                    &abs_path,
                    ctx.config.tracking.preserve_permissions,
                    false,
                )?;

                // Calculate new hash for index
                let (new_hash, _cache) = crate::storage::file_ops::hash_file(&abs_path, None)?;
                let metadata = fs::metadata(&abs_path)?;

                // Stage the restored file
                index.stage_entry(FileEntry {
                    path: path.clone(),
                    hash: new_hash,
                    size: metadata.len(),
                    modified: i64::try_from(
                        metadata
                            .modified()?
                            .duration_since(std::time::UNIX_EPOCH)?
                            .as_secs(),
                    )
                    .unwrap_or(i64::MAX),
                    mode: *mode,
                    cached_hash: None,
                });
            }
        }
    }

    // Save updated index
    index.save(&index_path)?;

    Ok(())
}

/// Create a new commit representing the revert operation
///
/// Generates a commit that captures the state after reverting changes,
/// with the specified commit message.
///
/// # Arguments
///
/// * `ctx` - The dotman context containing repository configuration
/// * `message` - The commit message for the revert commit
///
/// # Errors
///
/// Returns an error if:
/// - Index file cannot be loaded
/// - Commit ID generation fails
/// - Snapshot creation fails
/// - HEAD update fails
fn create_revert_commit(ctx: &DotmanContext, message: &str) -> Result<()> {
    let index_path = ctx.repo_path.join(INDEX_FILE);
    let index = Index::load(&index_path)?;

    // Get timestamp and author for commit
    let (timestamp, nanos) = get_precise_timestamp();
    let author = get_user_from_config(&ctx.config);

    let resolver = RefResolver::new(ctx.repo_path.clone());
    let parent = resolver.resolve("HEAD").ok();

    // Create tree hash from all staged file hashes
    let mut tree_content = String::new();
    for (path, entry) in &index.staged_entries {
        #[allow(clippy::expect_used)] // Writing to String never fails
        writeln!(&mut tree_content, "{} {}", entry.hash, path.display())
            .expect("String write should never fail");
    }
    let tree_hash = hash_bytes(tree_content.as_bytes());

    let parents: Vec<String> = parent.into_iter().collect();
    let parent_refs: Vec<&str> = parents.iter().map(String::as_str).collect();

    // Generate content-addressed commit ID
    let commit_id =
        generate_commit_id(&tree_hash, &parent_refs, message, &author, timestamp, nanos);

    // Create commit object
    let commit = Commit {
        id: commit_id.clone(),
        parents,
        message: message.to_string(),
        author,
        timestamp,
        tree_hash,
    };

    // Create snapshot
    let snapshot_manager = SnapshotManager::with_permissions(
        ctx.repo_path.clone(),
        ctx.config.core.compression_level,
        ctx.config.tracking.preserve_permissions,
    );

    let files: Vec<FileEntry> = index.staged_entries.values().cloned().collect();
    snapshot_manager.create_snapshot(commit, &files, None::<fn(usize)>)?;

    // Clear staging area after creating commit
    let mut index = index;
    index.commit_staged();
    index.save(&index_path)?;

    // Update HEAD
    update_head(ctx, &commit_id)?;

    Ok(())
}

/// Update HEAD to point to the new revert commit
///
/// # Arguments
///
/// * `ctx` - The dotman context containing repository configuration
/// * `commit_id` - The ID of the new revert commit
///
/// # Errors
///
/// Returns an error if the HEAD reference cannot be updated
fn update_head(ctx: &DotmanContext, commit_id: &str) -> Result<()> {
    use crate::refs::RefManager;

    let ref_manager = RefManager::new(ctx.repo_path.clone());
    let message = format!("revert: {commit_id}");
    ref_manager.set_head_to_commit(commit_id, Some("revert"), Some(&message))
}

/// Represents a change operation needed to revert a commit
#[derive(Debug, Clone)]
enum RevertChange {
    /// Delete a file that was added in the original commit
    Delete(PathBuf),
    /// Restore a file to its previous state
    Restore {
        /// Path to the file being restored
        path: PathBuf,
        /// Content hash of the file's previous version
        content_hash: String,
        /// Unix file mode/permissions
        mode: u32,
    },
}
