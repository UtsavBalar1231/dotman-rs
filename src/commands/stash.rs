use crate::commands::status::get_current_files;
use crate::output;
use crate::refs::RefManager;
use crate::storage::FileEntry;
use crate::storage::FileStatus;
use crate::storage::file_ops::hash_file;
use crate::storage::index::Index;
use crate::storage::stash::{StashEntry, StashFile, StashManager};
use crate::utils::pager::PagerOutput;
use crate::{DotmanContext, INDEX_FILE};
use anyhow::{Context, Result};
use colored::Colorize;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// Stash subcommands for managing temporary changes
#[derive(Debug, Clone)]
pub enum StashCommand {
    /// Save current changes to stash
    Push {
        /// Optional description for the stash
        message: Option<String>,
        /// Whether to include untracked files
        include_untracked: bool,
        /// Whether to keep staged changes in index
        keep_index: bool,
    },
    /// Apply and remove most recent stash
    Pop,
    /// Apply stash without removing it
    Apply {
        /// Specific stash to apply (or most recent if None)
        stash_id: Option<String>,
    },
    /// List all stashes
    List,
    /// Show contents of a stash
    Show {
        /// Specific stash to show (or most recent if None)
        stash_id: Option<String>,
    },
    /// Delete a specific stash
    Drop {
        /// ID of stash to delete
        stash_id: String,
    },
    /// Delete all stashes
    Clear,
}

/// Main stash command entry point
///
/// # Errors
///
/// Returns an error if:
/// - The repository is not initialized
/// - Stash operations fail (push, pop, apply, drop)
/// - The specified stash does not exist
/// - Conflicts occur when applying stashed changes
pub fn execute(ctx: &DotmanContext, command: StashCommand) -> Result<()> {
    ctx.check_repo_initialized()?;

    match command {
        StashCommand::Push {
            message,
            include_untracked,
            keep_index,
        } => push_stash(ctx, message, include_untracked, keep_index),
        StashCommand::Pop => pop_stash(ctx),
        StashCommand::Apply { stash_id } => apply_stash(ctx, stash_id, false),
        StashCommand::List => list_stashes(ctx),
        StashCommand::Show { stash_id } => show_stash(ctx, stash_id),
        StashCommand::Drop { stash_id } => drop_stash(ctx, &stash_id),
        StashCommand::Clear => clear_stashes(ctx),
    }
}

/// Push current changes to stash
#[allow(clippy::too_many_lines)] // Complex command handling staged/unstaged changes, tracking manifest, and stash operations
fn push_stash(
    ctx: &DotmanContext,
    message: Option<String>,
    include_untracked: bool,
    keep_index: bool,
) -> Result<()> {
    let index_path = ctx.repo_path.join(INDEX_FILE);
    let index = Index::load(&index_path)?;

    // Get all current files
    let current_files = get_current_files(ctx)?;

    // Get file statuses
    let mut statuses = index.get_status_parallel(&current_files);

    // Add untracked files if requested
    if include_untracked {
        let untracked = find_untracked_files(ctx, &index)?;
        for file in untracked {
            statuses.push(FileStatus::Untracked(file));
        }
    }

    if statuses.is_empty() {
        output::info("No local changes to save");
        return Ok(());
    }

    let ref_manager = RefManager::new(ctx.repo_path.clone());
    let head_commit = ref_manager
        .get_head_commit()?
        .context("No commits yet - cannot stash")?;

    // Create stash manager
    let stash_manager = StashManager::new(ctx.repo_path.clone(), ctx.config.core.compression_level);

    // Build stash entry
    let stash_id = stash_manager.generate_stash_id()?;
    let message = message.unwrap_or_else(|| {
        format!(
            "WIP on {}",
            get_branch_name(&ref_manager).unwrap_or_else(|_| "HEAD".to_string())
        )
    });

    let home = dirs::home_dir().context("Could not find home directory")?;

    // Collect files to stash
    let mut stash_files = HashMap::new();
    for status in &statuses {
        let path = status.path();
        let abs_path = if path.is_relative() {
            home.join(path)
        } else {
            path.to_path_buf()
        };

        match status {
            FileStatus::Added(p) | FileStatus::Modified(p) | FileStatus::Untracked(p) => {
                if abs_path.exists() {
                    let content = fs::read(&abs_path)?;
                    let (hash, _cache) = hash_file(&abs_path, None)?;
                    let metadata = fs::metadata(&abs_path)?;
                    let mode = get_file_mode(&metadata);

                    stash_files.insert(
                        p.clone(),
                        StashFile {
                            hash,
                            mode,
                            status: status.clone(),
                            content: Some(content),
                        },
                    );
                }
            }
            FileStatus::Deleted(p) => {
                // For deleted files, we just record the deletion
                stash_files.insert(
                    p.clone(),
                    StashFile {
                        hash: String::new(),
                        mode: 0,
                        status: status.clone(),
                        content: None,
                    },
                );
            }
        }
    }

    // Load HEAD snapshot to get committed files for index_state
    let snapshot_manager = crate::storage::snapshots::SnapshotManager::with_permissions(
        ctx.repo_path.clone(),
        ctx.config.core.compression_level,
        ctx.config.tracking.preserve_permissions,
    );
    let snapshot = snapshot_manager.load_snapshot(&head_commit)?;

    // Convert snapshot files to FileEntry for index_state
    let index_state: Vec<FileEntry> = snapshot
        .files
        .iter()
        .map(|(path, snap_file)| {
            // For stashed index state, we don't need exact size/modified time
            // since we have the hash and mode which are the critical fields
            FileEntry {
                path: path.clone(),
                hash: snap_file.hash.clone(),
                size: 0,     // Not critical for stash restore
                modified: 0, // Not critical for stash restore
                mode: snap_file.mode,
                cached_hash: None,
            }
        })
        .collect();

    // Create stash entry
    let stash_entry = StashEntry {
        id: stash_id,
        message: message.clone(),
        timestamp: crate::utils::get_current_timestamp(),
        parent_commit: head_commit,
        files: stash_files,
        index_state,
    };

    // Save stash
    stash_manager.save_stash(&stash_entry)?;

    output::success(&format!(
        "Saved working directory and index state: {}",
        message.dimmed()
    ));

    // Reset working directory to HEAD state (unless --keep-index)
    if !keep_index {
        // Reset modified files to their HEAD state
        reset_to_head(ctx)?;

        // Remove untracked files that were stashed
        if include_untracked {
            for (path, file) in &stash_entry.files {
                if matches!(file.status, FileStatus::Untracked(_)) {
                    let abspath = if path.is_relative() {
                        home.join(path)
                    } else {
                        path.clone()
                    };
                    if abspath.exists() {
                        fs::remove_file(&abspath)?;
                    }
                }
            }
        }
    }

    println!("HEAD is now at {}", get_current_commit_info(ctx)?);

    Ok(())
}

/// Pop the latest stash and apply it
fn pop_stash(ctx: &DotmanContext) -> Result<()> {
    let stash_manager = StashManager::new(ctx.repo_path.clone(), ctx.config.core.compression_level);

    // Get latest stash ID
    let stash_id = stash_manager
        .get_latest_stash_id()?
        .context("No stash entries found")?;

    // Apply the stash
    apply_stash(ctx, Some(stash_id.clone()), true)?;

    stash_manager.pop_from_stack()?;

    // Delete stash file
    stash_manager.delete_stash(&stash_id)?;

    output::success("Dropped stash entry");

    Ok(())
}

/// Apply a stash without removing it
fn apply_stash(ctx: &DotmanContext, stash_id: Option<String>, is_pop: bool) -> Result<()> {
    let stash_manager = StashManager::new(ctx.repo_path.clone(), ctx.config.core.compression_level);

    // Get stash ID
    let stash_id = match stash_id {
        Some(id) => id,
        None => stash_manager
            .get_latest_stash_id()?
            .context("No stash entries found")?,
    };

    // Load stash entry
    let stash = stash_manager.load_stash(&stash_id)?;

    let ref_manager = RefManager::new(ctx.repo_path.clone());
    let current_commit = ref_manager.get_head_commit()?.unwrap_or_default();

    if current_commit != stash.parent_commit {
        output::warning(&format!(
            "Stash was created on commit {}, but you are on {}",
            &stash.parent_commit[..8.min(stash.parent_commit.len())],
            &current_commit[..8.min(current_commit.len())]
        ));
    }

    let home = dirs::home_dir().context("Could not find home directory")?;

    // Apply stashed files
    let mut applied = 0;
    let mut conflicts = 0;

    let total_files = stash.files.len();
    let mut progress = output::start_progress("Applying stashed files", total_files);

    for (i, (path, stash_file)) in stash.files.iter().enumerate() {
        let abs_path = if path.is_relative() {
            home.join(path)
        } else {
            path.clone()
        };

        match &stash_file.status {
            FileStatus::Added(_) | FileStatus::Modified(_) | FileStatus::Untracked(_) => {
                if let Some(content) = &stash_file.content {
                    // If we're on the parent commit, the file was just reset by the stash push
                    // so we can safely overwrite it
                    if abs_path.exists() && current_commit != stash.parent_commit {
                        let (current_hash, _cache) = hash_file(&abs_path, None)?;
                        if current_hash != stash_file.hash {
                            output::warning(&format!(
                                "Conflict in {}: file has been modified since stash",
                                path.display()
                            ));
                            conflicts += 1;
                            continue;
                        }
                    }

                    // Create parent directories if needed
                    if let Some(parent) = abs_path.parent() {
                        fs::create_dir_all(parent)?;
                    }

                    // Write file
                    fs::write(&abs_path, content)?;

                    // Set permissions using cross-platform module
                    let permissions =
                        crate::utils::permissions::FilePermissions::from_mode(stash_file.mode);
                    permissions
                        .apply_to_path(&abs_path, ctx.config.tracking.preserve_permissions)?;

                    applied += 1;
                }
            }
            FileStatus::Deleted(_) => {
                if abs_path.exists() {
                    fs::remove_file(&abs_path)?;
                    applied += 1;
                }
            }
        }
        progress.update(i + 1);
    }

    progress.finish();

    // Update index if needed
    if !is_pop {
        // For apply (not pop), we might want to update the index
        // This depends on the specific requirements
    }

    if conflicts > 0 {
        output::warning(&format!(
            "Applied stash with {conflicts} conflicts. Please resolve them manually."
        ));
    } else {
        output::success(&format!("Applied {applied} changes from stash"));
    }

    Ok(())
}

/// List all stashes
fn list_stashes(ctx: &DotmanContext) -> Result<()> {
    let stash_manager = StashManager::new(ctx.repo_path.clone(), ctx.config.core.compression_level);
    let stashes = stash_manager.list_stashes()?;

    if stashes.is_empty() {
        println!("No stash entries found");
        return Ok(());
    }

    println!("{}", "Stash entries:".bold());
    for (i, stash_id) in stashes.iter().enumerate() {
        let stash = stash_manager.load_stash(stash_id)?;
        let timestamp = chrono::DateTime::from_timestamp(stash.timestamp, 0).map_or_else(
            || "Unknown time".to_string(),
            |dt| dt.format("%Y-%m-%d %H:%M:%S").to_string(),
        );

        println!(
            "  stash@{{{}}}: {} - {} ({})",
            i,
            stash_id[..16.min(stash_id.len())].dimmed(),
            stash.message,
            timestamp.dimmed()
        );
    }

    Ok(())
}

/// Show the contents of a stash
fn show_stash(ctx: &DotmanContext, stash_id: Option<String>) -> Result<()> {
    let stash_manager = StashManager::new(ctx.repo_path.clone(), ctx.config.core.compression_level);

    // Get stash ID
    let stash_id = match stash_id {
        Some(id) => id,
        None => stash_manager
            .get_latest_stash_id()?
            .context("No stash entries found")?,
    };

    // Load stash
    let stash = stash_manager.load_stash(&stash_id)?;

    let mut output = PagerOutput::default();
    output.appendln(&format!("{}", "Stash contents:".bold()));
    output.appendln(&format!("  ID: {}", stash.id.dimmed()));
    output.appendln(&format!("  Message: {}", stash.message));
    output.appendln(&format!("  Parent: {}", stash.parent_commit.dimmed()));
    output.appendln(&format!(
        "  Created: {}",
        chrono::DateTime::from_timestamp(stash.timestamp, 0).map_or_else(
            || "Unknown".to_string(),
            |dt| dt.format("%Y-%m-%d %H:%M:%S").to_string()
        )
    ));
    output.appendln("");

    // Group files by status
    let mut added = Vec::new();
    let mut modified = Vec::new();
    let mut deleted = Vec::new();
    let mut untracked = Vec::new();

    for file in stash.files.values() {
        match &file.status {
            FileStatus::Added(p) => added.push(p),
            FileStatus::Modified(p) => modified.push(p),
            FileStatus::Deleted(p) => deleted.push(p),
            FileStatus::Untracked(p) => untracked.push(p),
        }
    }

    if !added.is_empty() {
        output.appendln(&format!("{}", "Added files:".green().bold()));
        for path in added {
            output.appendln(&format!("  + {}", path.display()));
        }
    }

    if !modified.is_empty() {
        output.appendln(&format!("{}", "Modified files:".yellow().bold()));
        for path in modified {
            output.appendln(&format!("  ~ {}", path.display()));
        }
    }

    if !deleted.is_empty() {
        output.appendln(&format!("{}", "Deleted files:".red().bold()));
        for path in deleted {
            output.appendln(&format!("  - {}", path.display()));
        }
    }

    if !untracked.is_empty() {
        output.appendln(&format!("{}", "Untracked files:".blue().bold()));
        for path in untracked {
            output.appendln(&format!("  ? {}", path.display()));
        }
    }

    output.show()?;

    Ok(())
}

/// Drop a specific stash
fn drop_stash(ctx: &DotmanContext, stash_id: &str) -> Result<()> {
    let stash_manager = StashManager::new(ctx.repo_path.clone(), ctx.config.core.compression_level);

    stash_manager.delete_stash(stash_id)?;
    output::success(&format!("Dropped stash {stash_id}"));

    Ok(())
}

/// Clear all stashes
fn clear_stashes(ctx: &DotmanContext) -> Result<()> {
    let stash_manager = StashManager::new(ctx.repo_path.clone(), ctx.config.core.compression_level);

    stash_manager.clear_all_stashes()?;
    output::success("Cleared all stash entries");

    Ok(())
}

// Helper functions

/// Get the current branch name
fn get_branch_name(ref_manager: &RefManager) -> Result<String> {
    ref_manager
        .current_branch()
        .map(|b| b.unwrap_or_else(|| "HEAD".to_string()))
}

/// Get current commit info for display
fn get_current_commit_info(ctx: &DotmanContext) -> Result<String> {
    use crate::storage::snapshots::SnapshotManager;

    let ref_manager = RefManager::new(ctx.repo_path.clone());
    let commit_id = ref_manager.get_head_commit()?.context("No commits yet")?;

    let snapshot_manager = SnapshotManager::with_permissions(
        ctx.repo_path.clone(),
        ctx.config.core.compression_level,
        ctx.config.tracking.preserve_permissions,
    );
    let snapshot = snapshot_manager.load_snapshot(&commit_id)?;

    Ok(format!(
        "{} {}",
        commit_id[..8.min(commit_id.len())].yellow(),
        snapshot.commit.message.lines().next().unwrap_or("")
    ))
}

/// Reset working directory to HEAD state
fn reset_to_head(ctx: &DotmanContext) -> Result<()> {
    use crate::commands::checkout;

    // Use checkout to reset to HEAD
    checkout::execute(ctx, "HEAD", true)?;

    Ok(())
}

/// Find untracked files
fn find_untracked_files(ctx: &DotmanContext, index: &Index) -> Result<Vec<PathBuf>> {
    use crate::commands::status::find_untracked_files as status_find_untracked;
    status_find_untracked(ctx, index)
}

/// Get file mode from metadata
#[cfg(unix)]
fn get_file_mode(metadata: &std::fs::Metadata) -> u32 {
    use std::os::unix::fs::MetadataExt;
    metadata.mode()
}

#[cfg(not(unix))]
fn get_file_mode(_metadata: &std::fs::Metadata) -> u32 {
    panic!("File mode retrieval not implemented for this platform")
}
