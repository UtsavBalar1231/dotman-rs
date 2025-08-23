use crate::commands::status::get_current_files;
use crate::refs::RefManager;
use crate::storage::FileStatus;
use crate::storage::file_ops::hash_file;
use crate::storage::index::{ConcurrentIndex, Index};
use crate::storage::stash::{StashEntry, StashFile, StashManager};
use crate::utils::pager::PagerOutput;
use crate::{DotmanContext, INDEX_FILE};
use anyhow::{Context, Result};
use colored::Colorize;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// Stash subcommands
#[derive(Debug, Clone)]
pub enum StashCommand {
    Push {
        message: Option<String>,
        include_untracked: bool,
        keep_index: bool,
    },
    Pop,
    Apply {
        stash_id: Option<String>,
    },
    List,
    Show {
        stash_id: Option<String>,
    },
    Drop {
        stash_id: String,
    },
    Clear,
}

/// Main stash command entry point
pub fn execute(ctx: &DotmanContext, command: StashCommand) -> Result<()> {
    ctx.ensure_repo_exists()?;

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
        StashCommand::Drop { stash_id } => drop_stash(ctx, stash_id),
        StashCommand::Clear => clear_stashes(ctx),
    }
}

/// Push current changes to stash
fn push_stash(
    ctx: &DotmanContext,
    message: Option<String>,
    include_untracked: bool,
    keep_index: bool,
) -> Result<()> {
    let index_path = ctx.repo_path.join(INDEX_FILE);
    let index = Index::load(&index_path)?;
    let concurrent_index = ConcurrentIndex::from_index(index.clone());

    // Get all current files
    let current_files = get_current_files(ctx)?;

    // Get file statuses
    let mut statuses = concurrent_index.get_status_parallel(&current_files);

    // Add untracked files if requested
    if include_untracked {
        let untracked = find_untracked_files(ctx, &index)?;
        for file in untracked {
            statuses.push(FileStatus::Untracked(file));
        }
    }

    // Check if there are any changes to stash
    if statuses.is_empty() {
        super::print_info("No local changes to save");
        return Ok(());
    }

    // Get current HEAD commit
    let ref_manager = RefManager::new(ctx.repo_path.clone());
    let head_commit = ref_manager
        .get_head_commit()?
        .context("No commits yet - cannot stash")?;

    // Create stash manager
    let stash_manager = StashManager::new(ctx.repo_path.clone(), ctx.config.core.compression_level);

    // Build stash entry
    let stash_id = stash_manager.generate_stash_id();
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
                    let hash = hash_file(&abs_path)?;
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

    // Create stash entry
    let stash_entry = StashEntry {
        id: stash_id.clone(),
        message: message.clone(),
        timestamp: crate::utils::get_current_timestamp(),
        parent_commit: head_commit,
        files: stash_files,
        index_state: index.entries.values().cloned().collect(),
    };

    // Save stash
    stash_manager.save_stash(&stash_entry)?;

    super::print_success(&format!(
        "Saved working directory and index state: {}",
        message.dimmed()
    ));

    // Reset working directory to HEAD state (unless --keep-index)
    if !keep_index {
        // Reset modified files to their HEAD state
        reset_to_head(ctx)?;
        
        // Remove untracked files that were stashed
        if include_untracked {
            for (_path, file) in &stash_entry.files {
                if matches!(file.status, FileStatus::Untracked(_)) {
                    let abs_path = if _path.is_relative() {
                        home.join(_path)
                    } else {
                        _path.clone()
                    };
                    if abs_path.exists() {
                        fs::remove_file(&abs_path)?;
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

    // Remove from stack
    stash_manager.pop_from_stack()?;

    // Delete stash file
    stash_manager.delete_stash(&stash_id)?;

    super::print_success("Dropped stash entry");

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

    // Check if we're on a compatible commit
    let ref_manager = RefManager::new(ctx.repo_path.clone());
    let current_commit = ref_manager.get_head_commit()?.unwrap_or_default();

    if current_commit != stash.parent_commit {
        super::print_warning(&format!(
            "Stash was created on commit {}, but you are on {}",
            &stash.parent_commit[..8.min(stash.parent_commit.len())],
            &current_commit[..8.min(current_commit.len())]
        ));
    }

    let home = dirs::home_dir().context("Could not find home directory")?;

    // Apply stashed files
    let mut applied = 0;
    let mut conflicts = 0;

    for (path, stash_file) in &stash.files {
        let abs_path = if path.is_relative() {
            home.join(path)
        } else {
            path.clone()
        };

        match &stash_file.status {
            FileStatus::Added(_) | FileStatus::Modified(_) | FileStatus::Untracked(_) => {
                if let Some(content) = &stash_file.content {
                    // Check for conflicts only if not on the same commit
                    // If we're on the parent commit, the file was just reset by the stash push
                    // so we can safely overwrite it
                    if abs_path.exists() && current_commit != stash.parent_commit {
                        let current_hash = hash_file(&abs_path)?;
                        if current_hash != stash_file.hash {
                            super::print_warning(&format!(
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

                    // Set permissions
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        let permissions = std::fs::Permissions::from_mode(stash_file.mode);
                        fs::set_permissions(&abs_path, permissions)?;
                    }

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
    }

    // Update index if needed
    if !is_pop {
        // For apply (not pop), we might want to update the index
        // This depends on the specific requirements
    }

    if conflicts > 0 {
        super::print_warning(&format!(
            "Applied stash with {} conflicts. Please resolve them manually.",
            conflicts
        ));
    } else {
        super::print_success(&format!("Applied {} changes from stash", applied));
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
        let timestamp = chrono::DateTime::from_timestamp(stash.timestamp, 0)
            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or_else(|| "Unknown time".to_string());

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

    let mut output = PagerOutput::new();
    output.appendln(&format!("{}", "Stash contents:".bold()));
    output.appendln(&format!("  ID: {}", stash.id.dimmed()));
    output.appendln(&format!("  Message: {}", stash.message));
    output.appendln(&format!("  Parent: {}", stash.parent_commit.dimmed()));
    output.appendln(&format!(
        "  Created: {}",
        chrono::DateTime::from_timestamp(stash.timestamp, 0)
            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or_else(|| "Unknown".to_string())
    ));
    output.appendln("");

    // Group files by status
    let mut added = Vec::new();
    let mut modified = Vec::new();
    let mut deleted = Vec::new();
    let mut untracked = Vec::new();

    for (_path, file) in &stash.files {
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
fn drop_stash(ctx: &DotmanContext, stash_id: String) -> Result<()> {
    let stash_manager = StashManager::new(ctx.repo_path.clone(), ctx.config.core.compression_level);

    stash_manager.delete_stash(&stash_id)?;
    super::print_success(&format!("Dropped stash {}", stash_id));

    Ok(())
}

/// Clear all stashes
fn clear_stashes(ctx: &DotmanContext) -> Result<()> {
    let stash_manager = StashManager::new(ctx.repo_path.clone(), ctx.config.core.compression_level);

    stash_manager.clear_all_stashes()?;
    super::print_success("Cleared all stash entries");

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

    let snapshot_manager =
        SnapshotManager::new(ctx.repo_path.clone(), ctx.config.core.compression_level);
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
    0o644 // Default file mode for non-Unix systems
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stash_command_variants() {
        // Test that all command variants can be created
        let _ = StashCommand::Push {
            message: Some("test".to_string()),
            include_untracked: true,
            keep_index: false,
        };
        let _ = StashCommand::Pop;
        let _ = StashCommand::Apply {
            stash_id: Some("stash_123".to_string()),
        };
        let _ = StashCommand::List;
        let _ = StashCommand::Show { stash_id: None };
        let _ = StashCommand::Drop {
            stash_id: "stash_123".to_string(),
        };
        let _ = StashCommand::Clear;
    }
}
