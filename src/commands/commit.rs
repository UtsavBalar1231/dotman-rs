use crate::DotmanContext;
use crate::commands::context::CommandContext;
use crate::refs::updater::ReflogUpdater;
use crate::storage::file_ops::hash_bytes;
use crate::storage::index::Index;
use crate::storage::{Commit, FileEntry};
use crate::utils::formatters::format_commit_id;
use crate::utils::{commit::generate_commit_id, get_precise_timestamp, get_user_from_config};
use anyhow::{Context, Result};
use colored::Colorize;
use std::path::PathBuf;

/// Execute commit command to create a new commit
///
/// # Errors
///
/// Returns an error if:
/// - Repository is not initialized
/// - No files are tracked or staged
/// - Failed to save index or create snapshot
pub fn execute(ctx: &DotmanContext, message: &str, all: bool) -> Result<()> {
    ctx.ensure_initialized()?;

    let index_path = ctx.repo_path.join("index.bin");
    let mut index = ctx.load_index()?;

    if index.staged_entries.is_empty() && index.entries.is_empty() {
        super::print_warning("No files tracked. Use 'dot add' to track files first.");
        anyhow::bail!("No files tracked");
    }

    if all {
        super::print_info("Staging all tracked files...");
        stage_all_tracked_files(ctx, &mut index)?;
    }

    if !index.has_staged_changes() {
        super::print_warning(
            "No changes staged for commit. Use 'dot add' to stage changes or 'dot commit --all' to commit all changes.",
        );
        anyhow::bail!("No changes staged for commit");
    }

    let (timestamp, nanos) = get_precise_timestamp();
    let author = get_user_from_config(&ctx.config);

    let parent = get_last_commit_id(ctx)?;

    let mut tree_content = String::new();
    for (path, entry) in &index.staged_entries {
        use std::fmt::Write;
        let _ = writeln!(&mut tree_content, "{} {}", entry.hash, path.display());
    }
    // Include deletions in the tree hash (marked with a special hash)
    for path in &index.deleted_entries {
        use std::fmt::Write;
        let _ = writeln!(&mut tree_content, "DELETED {}", path.display());
    }
    let tree_hash = hash_bytes(tree_content.as_bytes());

    let commit_id = generate_commit_id(
        &tree_hash,
        parent.as_deref(),
        message,
        &author,
        timestamp,
        nanos,
    );

    let commit = Commit {
        id: commit_id.clone(),
        parent,
        message: message.to_string(),
        author,
        timestamp,
        tree_hash,
    };

    let snapshot_manager = ctx.create_snapshot_manager();

    // Merge entries + staged_entries to get the complete state after commit
    // This ensures the snapshot contains ALL tracked files, not just the staged changes
    let mut all_files = std::collections::HashMap::new();

    // Start with existing committed files
    for (path, entry) in &index.entries {
        if !index.deleted_entries.contains(path) {
            all_files.insert(path.clone(), entry.clone());
        }
    }

    // Override/add with staged changes
    for (path, entry) in &index.staged_entries {
        if !index.deleted_entries.contains(path) {
            all_files.insert(path.clone(), entry.clone());
        }
    }

    let files: Vec<FileEntry> = all_files.values().cloned().collect();
    snapshot_manager.create_snapshot(commit.clone(), &files)?;

    index.commit_staged();
    index.save(&index_path)?;

    update_head(ctx, &commit_id)?;

    let display_id = format_commit_id(&commit_id);
    super::print_success(&format!(
        "Committed {} with {} files",
        display_id.yellow(),
        files.len()
    ));
    println!("  {}: {}", "Author".bold(), commit.author);
    println!("  {}: {}", "Message".bold(), commit.message);

    Ok(())
}

/// Execute commit amend to modify the last commit
///
/// # Errors
///
/// Returns an error if:
/// - Repository is not initialized
/// - No commits exist to amend
/// - Failed to load or save changes
pub fn execute_amend(ctx: &DotmanContext, message: Option<&str>, all: bool) -> Result<()> {
    ctx.ensure_initialized()?;

    let resolver = ctx.create_ref_resolver();
    let last_commit_id = resolver.resolve("HEAD").context("No commits to amend")?;

    let snapshot_manager = ctx.create_snapshot_manager();
    let last_snapshot = snapshot_manager
        .load_snapshot(&last_commit_id)
        .with_context(|| format!("Failed to load commit: {last_commit_id}"))?;

    let index_path = ctx.repo_path.join("index.bin");
    let mut index = ctx.load_index()?;

    if all {
        super::print_info("Staging all tracked files...");
        stage_all_tracked_files(ctx, &mut index)?;
    }

    if index.staged_entries.is_empty() {
        super::print_warning("No files staged. Use 'dot add' to stage files first.");
        return Ok(());
    }

    let commit_message = message.unwrap_or(&last_snapshot.commit.message);

    let mut tree_content = String::new();
    for (path, entry) in &index.staged_entries {
        use std::fmt::Write;
        let _ = writeln!(&mut tree_content, "{} {}", entry.hash, path.display());
    }
    // Include deletions in the tree hash (marked with a special hash)
    for path in &index.deleted_entries {
        use std::fmt::Write;
        let _ = writeln!(&mut tree_content, "DELETED {}", path.display());
    }
    let tree_hash = hash_bytes(tree_content.as_bytes());

    let (timestamp, nanos) = get_precise_timestamp();
    let author = get_user_from_config(&ctx.config);

    let commit_id = generate_commit_id(
        &tree_hash,
        last_snapshot.commit.parent.as_deref(),
        commit_message,
        &author,
        timestamp,
        nanos,
    );

    let commit = Commit {
        id: commit_id.clone(),
        parent: last_snapshot.commit.parent.clone(),
        message: commit_message.to_string(),
        author,
        timestamp,
        tree_hash,
    };

    // Delete the old snapshot
    snapshot_manager.delete_snapshot(&last_commit_id)?;

    let files: Vec<FileEntry> = index.staged_entries.values().cloned().collect();
    snapshot_manager.create_snapshot(commit.clone(), &files)?;

    index.commit_staged();
    index.save(&index_path)?;

    // Update HEAD to point to the new commit ID since it's content-addressed
    update_head(ctx, &commit_id)?;

    let display_id = format_commit_id(&commit_id);

    super::print_success(&format!(
        "Amended commit {} with {} files",
        display_id.yellow(),
        files.len()
    ));
    println!("  {}: {}", "Author".bold(), commit.author);
    println!("  {}: {}", "Message".bold(), commit.message);

    Ok(())
}

/// Stage all tracked files for commit
///
/// # Errors
///
/// Returns an error if failed to find home directory or create file entries
fn stage_all_tracked_files(ctx: &DotmanContext, index: &mut Index) -> Result<()> {
    // Get home directory for making paths relative
    let home = ctx.get_home_dir()?;

    let mut staged = 0;

    // Stage all currently tracked files with their current state
    let entries: Vec<(PathBuf, FileEntry)> = index.entries.clone().into_iter().collect();
    for (path, existing_entry) in entries {
        // Need to convert relative path back to absolute for checking existence
        let abs_path = home.join(&path);
        if abs_path.exists() {
            // Use cached hash from existing entry for performance
            let entry = crate::commands::add::create_file_entry(
                &abs_path,
                &home,
                existing_entry.cached_hash.as_ref(),
            )
            .unwrap_or_else(|_| {
                // Fallback to non-cached if there's an error
                crate::commands::add::create_file_entry(&abs_path, &home, None)
                    .unwrap_or(existing_entry)
            });
            index.stage_entry(entry);
            staged += 1;
        }
    }

    if staged > 0 {
        super::print_info(&format!("Staged {staged} tracked file(s)"));
    }

    Ok(())
}

/// Get the ID of the last commit
///
/// # Errors
///
/// Returns an error if failed to read HEAD
fn get_last_commit_id(ctx: &DotmanContext) -> Result<Option<String>> {
    use crate::refs::RefManager;

    let ref_manager = RefManager::new(ctx.repo_path.clone());
    ref_manager.get_head_commit()
}

/// Update HEAD to point to a new commit
///
/// # Errors
///
/// Returns an error if failed to update HEAD or reflog
fn update_head(ctx: &DotmanContext, commit_id: &str) -> Result<()> {
    let updater = ReflogUpdater::new(ctx.repo_path.clone());
    updater.commit_head(commit_id, format_commit_id(commit_id))
}
