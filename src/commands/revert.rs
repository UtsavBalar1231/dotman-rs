use crate::refs::resolver::RefResolver;
use crate::storage::index::{Index, IndexDiffer};
use crate::storage::snapshots::SnapshotManager;
use crate::storage::{Commit, FileEntry, FileStatus};
use crate::utils::{
    commit::generate_commit_id, get_current_timestamp, get_current_user_with_config,
    hash::hash_bytes,
};
use crate::{DotmanContext, INDEX_FILE};
use anyhow::{Context, Result};
use colored::Colorize;
use std::fs;
use std::path::PathBuf;

pub fn execute(ctx: &DotmanContext, commit_ref: &str, no_edit: bool, force: bool) -> Result<()> {
    ctx.check_repo_initialized()?;

    if !force {
        let status_output = check_working_directory_clean(ctx)?;
        if !status_output {
            anyhow::bail!(
                "You have uncommitted changes. Use --force to override or commit your changes first."
            );
        }
    }

    // Resolve the commit reference
    let resolver = RefResolver::new(ctx.repo_path.clone());
    let target_commit_id = resolver
        .resolve(commit_ref)
        .with_context(|| format!("Failed to resolve commit reference: {}", commit_ref))?;

    let snapshot_manager =
        SnapshotManager::new(ctx.repo_path.clone(), ctx.config.core.compression_level);

    let target_snapshot = snapshot_manager
        .load_snapshot(&target_commit_id)
        .with_context(|| format!("Failed to load commit: {}", target_commit_id))?;

    let display_target = if target_commit_id.len() >= 8 {
        &target_commit_id[..8]
    } else {
        &target_commit_id
    };

    super::print_info(&format!(
        "Reverting commit {} \"{}\"",
        display_target.yellow(),
        target_snapshot.commit.message
    ));

    // Calculate what changes need to be reverted
    let changes_to_revert = calculate_revert_changes(ctx, &target_snapshot, &snapshot_manager)?;

    if changes_to_revert.is_empty() {
        super::print_info("No changes to revert.");
        return Ok(());
    }

    // Show what will be reverted
    display_revert_summary(&changes_to_revert);

    // Apply the inverse changes to the working directory and index
    apply_revert_changes(ctx, &changes_to_revert, &snapshot_manager)?;

    let revert_message = if no_edit {
        format!("Revert \"{}\"", target_snapshot.commit.message)
    } else {
        // In a real implementation, you might want to open an editor here
        // For now, we'll just use the default message
        format!("Revert \"{}\"", target_snapshot.commit.message)
    };

    create_revert_commit(ctx, &revert_message)?;

    super::print_success(&format!(
        "Reverted commit {} - \"{}\"",
        display_target.yellow(),
        target_snapshot.commit.message
    ));

    Ok(())
}

fn check_working_directory_clean(ctx: &DotmanContext) -> Result<bool> {
    use crate::commands::status::get_current_files;
    use crate::storage::index::ConcurrentIndex;

    let index_path = ctx.repo_path.join(INDEX_FILE);
    let index = Index::load(&index_path)?;
    let concurrent_index = ConcurrentIndex::from_index(index);

    let current_files = get_current_files(ctx)?;
    let statuses = concurrent_index.get_status_parallel(&current_files);

    Ok(statuses.is_empty())
}

fn calculate_revert_changes(
    _ctx: &DotmanContext,
    target_snapshot: &crate::storage::snapshots::Snapshot,
    snapshot_manager: &SnapshotManager,
) -> Result<Vec<RevertChange>> {
    let mut revert_changes = Vec::new();

    // Convert target snapshot to index for comparison
    let mut target_index = Index::new();
    for (path, file) in &target_snapshot.files {
        target_index.add_entry(FileEntry {
            path: path.clone(),
            hash: file.hash.clone(),
            size: 0, // Size not needed for comparison
            modified: target_snapshot.commit.timestamp,
            mode: file.mode,
        });
    }

    if let Some(parent_id) = &target_snapshot.commit.parent {
        // Commit has a parent - compare with parent to see what the original commit did
        let parent_snapshot = snapshot_manager
            .load_snapshot(parent_id)
            .with_context(|| format!("Failed to load parent commit: {}", parent_id))?;

        let mut parent_index = Index::new();
        for (path, file) in &parent_snapshot.files {
            parent_index.add_entry(FileEntry {
                path: path.clone(),
                hash: file.hash.clone(),
                size: 0,
                modified: parent_snapshot.commit.timestamp,
                mode: file.mode,
            });
        }

        // Find what changed from parent to target
        let changes = IndexDiffer::diff(&parent_index, &target_index);

        // Create inverse operations
        for change in changes {
            match change {
                FileStatus::Added(path) => {
                    // Original commit added this file - revert by deleting it
                    revert_changes.push(RevertChange::Delete(path));
                }
                FileStatus::Modified(path) => {
                    // Original commit modified this file - revert by restoring parent version
                    if let Some(parent_entry) = parent_index.get_entry(&path) {
                        revert_changes.push(RevertChange::Restore {
                            path: path.clone(),
                            content_hash: parent_entry.hash.clone(),
                            mode: parent_entry.mode,
                        });
                    }
                }
                FileStatus::Deleted(path) => {
                    // Original commit deleted this file - revert by restoring it from parent
                    if let Some(_parent_entry) = parent_index.get_entry(&path)
                        && let Some(parent_file) = parent_snapshot.files.get(&path)
                    {
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

fn display_revert_summary(changes: &[RevertChange]) {
    println!();
    super::print_info("Changes to be reverted:");

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

fn apply_revert_changes(
    ctx: &DotmanContext,
    changes: &[RevertChange],
    snapshot_manager: &SnapshotManager,
) -> Result<()> {
    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;

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

                index.remove_entry(path);
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

                // Set file permissions on Unix systems
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let permissions = fs::Permissions::from_mode(*mode);
                    fs::set_permissions(&abs_path, permissions)?;
                }

                // Calculate new hash for index
                let new_hash = crate::storage::file_ops::hash_file(&abs_path)?;
                let metadata = fs::metadata(&abs_path)?;

                // Update index with restored file
                index.add_entry(FileEntry {
                    path: path.clone(),
                    hash: new_hash,
                    size: metadata.len(),
                    modified: metadata
                        .modified()?
                        .duration_since(std::time::UNIX_EPOCH)?
                        .as_secs() as i64,
                    mode: *mode,
                });
            }
        }
    }

    // Save updated index
    index.save(&index_path)?;

    Ok(())
}

fn create_revert_commit(ctx: &DotmanContext, message: &str) -> Result<()> {
    let index_path = ctx.repo_path.join(INDEX_FILE);
    let index = Index::load(&index_path)?;

    // Get timestamp and author for commit
    let timestamp = get_current_timestamp();
    let author = get_current_user_with_config(&ctx.config);

    let resolver = RefResolver::new(ctx.repo_path.clone());
    let parent = resolver.resolve("HEAD").ok();

    // Create tree hash from all file hashes
    let mut tree_content = String::new();
    for (path, entry) in &index.entries {
        tree_content.push_str(&format!("{} {}\n", entry.hash, path.display()));
    }
    let tree_hash = hash_bytes(tree_content.as_bytes());

    // Generate content-addressed commit ID
    let commit_id = generate_commit_id(&tree_hash, parent.as_deref(), message, &author, timestamp);

    // Create commit object
    let commit = Commit {
        id: commit_id.clone(),
        parent,
        message: message.to_string(),
        author,
        timestamp,
        tree_hash,
    };

    // Create snapshot
    let snapshot_manager =
        SnapshotManager::new(ctx.repo_path.clone(), ctx.config.core.compression_level);

    let files: Vec<FileEntry> = index.entries.values().cloned().collect();
    snapshot_manager.create_snapshot(commit.clone(), &files)?;

    // Update HEAD
    update_head(ctx, &commit_id)?;

    Ok(())
}

fn update_head(ctx: &DotmanContext, commit_id: &str) -> Result<()> {
    use crate::refs::RefManager;

    let ref_manager = RefManager::new(ctx.repo_path.clone());
    let message = format!("revert: {}", commit_id);
    ref_manager.set_head_to_commit_with_reflog(commit_id, "revert", &message)
}

#[derive(Debug, Clone)]
enum RevertChange {
    Delete(PathBuf),
    Restore {
        path: PathBuf,
        content_hash: String,
        mode: u32,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use std::fs;
    use tempfile::tempdir;

    fn setup_test_context() -> Result<(tempfile::TempDir, DotmanContext)> {
        let temp = tempdir()?;
        let repo_path = temp.path().join(".dotman");
        let config_path = temp.path().join("config.toml");

        // Create repo structure
        fs::create_dir_all(&repo_path)?;
        fs::create_dir_all(repo_path.join("commits"))?;
        fs::create_dir_all(repo_path.join("objects"))?;

        // Create empty index
        let index = Index::new();
        let index_path = repo_path.join("index.bin");
        index.save(&index_path)?;

        let mut config = Config::default();
        config.core.repo_path = repo_path.clone();
        config.save(&config_path)?;

        let ctx = DotmanContext {
            repo_path,
            config_path,
            config,
            no_pager: true,
        };

        Ok((temp, ctx))
    }

    #[test]
    #[serial_test::serial]
    fn test_execute_nonexistent_commit() -> Result<()> {
        let (_temp, ctx) = setup_test_context()?;

        // Set HOME for the test
        unsafe {
            std::env::set_var("HOME", _temp.path());
        }

        let result = execute(&ctx, "nonexistent", false, false);
        assert!(result.is_err());

        Ok(())
    }

    #[test]
    fn test_revert_change_enum() {
        use std::path::PathBuf;

        let delete_change = RevertChange::Delete(PathBuf::from("test.txt"));
        let restore_change = RevertChange::Restore {
            path: PathBuf::from("restore.txt"),
            content_hash: "abc123".to_string(),
            mode: 0o644,
        };

        match delete_change {
            RevertChange::Delete(_) => (),
            _ => panic!("Wrong variant"),
        }

        match restore_change {
            RevertChange::Restore { .. } => (),
            _ => panic!("Wrong variant"),
        }
    }
}
