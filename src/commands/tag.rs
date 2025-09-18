use crate::DotmanContext;
use crate::refs::RefManager;
use crate::refs::resolver::RefResolver;
use crate::storage::snapshots::SnapshotManager;
use anyhow::{Context, Result};
use chrono::{Local, TimeZone};
use colored::Colorize;

/// Create a new tag
///
/// # Errors
///
/// Returns an error if:
/// - The repository is not initialized
/// - The tag name is empty or invalid
/// - The tag already exists
/// - The specified commit does not exist
pub fn create(ctx: &DotmanContext, name: &str, commit: Option<&str>) -> Result<()> {
    ctx.check_repo_initialized()?;

    let ref_manager = RefManager::new(ctx.repo_path.clone());

    // Validate tag name
    validate_tag_name(name)?;

    // Resolve and validate the commit
    let resolved_commit = if let Some(commit_ref) = commit {
        // Use RefResolver to handle HEAD, branches, short hashes, etc.
        let resolver = RefResolver::new(ctx.repo_path.clone());
        let commit_id = resolver
            .resolve(commit_ref)
            .with_context(|| format!("Failed to resolve commit reference: {commit_ref}"))?;

        // Verify the commit actually exists
        let snapshot_manager =
            SnapshotManager::new(ctx.repo_path.clone(), ctx.config.core.compression_level);
        if !snapshot_manager.snapshot_exists(&commit_id) {
            return Err(anyhow::anyhow!("Commit {commit_ref} does not exist"));
        }

        Some(commit_id)
    } else {
        // Default to HEAD - validate it exists and has commits
        let head_commit = ref_manager
            .get_head_commit()
            .context("Failed to get HEAD commit")?
            .ok_or_else(|| anyhow::anyhow!("No commits available to tag (repository is empty)"))?;

        // Verify HEAD commit exists
        let snapshot_manager =
            SnapshotManager::new(ctx.repo_path.clone(), ctx.config.core.compression_level);
        if !snapshot_manager.snapshot_exists(&head_commit) {
            return Err(anyhow::anyhow!("HEAD commit is corrupted or missing"));
        }

        Some(head_commit)
    };

    // Create the tag with the validated commit
    ref_manager.create_tag(name, resolved_commit.as_deref())?;

    let display_target = resolved_commit.as_ref().map_or("HEAD", |commit_id| {
        if commit_id.len() >= 8 {
            &commit_id[..8]
        } else {
            commit_id
        }
    });

    super::print_success(&format!("Created tag '{name}' at {display_target}"));
    Ok(())
}

/// Validate tag name for filesystem safety
fn validate_tag_name(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(anyhow::anyhow!("Tag name cannot be empty"));
    }

    // Check for invalid characters that could cause filesystem issues
    if name.contains('/') || name.contains('\\') || name.contains('\0') {
        return Err(anyhow::anyhow!("Tag name contains invalid characters"));
    }

    // Check for reserved names
    if name == "." || name == ".." || name == "HEAD" {
        return Err(anyhow::anyhow!("Tag name '{}' is reserved", name));
    }

    // Check for leading/trailing dots or spaces (problematic on some filesystems)
    if name.starts_with('.') || name.ends_with('.') || name.starts_with(' ') || name.ends_with(' ')
    {
        return Err(anyhow::anyhow!(
            "Tag name cannot start or end with dots or spaces"
        ));
    }

    Ok(())
}

/// List all tags
///
/// # Errors
///
/// Returns an error if:
/// - The repository is not initialized
/// - Cannot read tags from the repository
pub fn list(ctx: &DotmanContext) -> Result<()> {
    ctx.check_repo_initialized()?;

    let ref_manager = RefManager::new(ctx.repo_path.clone());
    let tags = ref_manager.list_tags()?;

    if tags.is_empty() {
        super::print_info("No tags exist");
        return Ok(());
    }

    let snapshot_manager =
        SnapshotManager::new(ctx.repo_path.clone(), ctx.config.core.compression_level);

    println!("{}", "Tags:".bold());
    for tag in tags {
        match ref_manager.get_tag_commit(&tag) {
            Ok(commit_id) => {
                let short_commit = &commit_id[..8.min(commit_id.len())];

                // Try to load the commit to get the message
                let message_preview = snapshot_manager.load_snapshot(&commit_id).ok().map_or_else(
                    || "<commit not found>".to_string(),
                    |snapshot| {
                        let msg = &snapshot.commit.message;
                        if msg.len() > 50 {
                            format!("{}...", &msg[..47])
                        } else {
                            msg.to_string()
                        }
                    },
                );

                println!(
                    "  {} -> {} {}",
                    tag.yellow(),
                    short_commit.dimmed(),
                    message_preview.dimmed()
                );
            }
            Err(_) => {
                // Tag exists but couldn't read commit (shouldn't happen)
                println!("  {} -> {}", tag.yellow(), "???".red());
            }
        }
    }

    Ok(())
}

/// Delete a tag
///
/// # Errors
///
/// Returns an error if:
/// - The repository is not initialized
/// - The tag does not exist
/// - The tag deletion fails
pub fn delete(ctx: &DotmanContext, name: &str, force: bool) -> Result<()> {
    ctx.check_repo_initialized()?;

    let ref_manager = RefManager::new(ctx.repo_path.clone());

    if !ref_manager.tag_exists(name) {
        return Err(anyhow::anyhow!("Tag '{}' does not exist", name));
    }

    // In a real implementation, we might want to check if the tag
    // points to an important commit and require force flag
    if !force {
        // For now, just show a warning
        super::print_warning(&format!("Deleting tag '{name}'"));
    }

    ref_manager.delete_tag(name)?;
    super::print_success(&format!("Deleted tag '{name}'"));

    Ok(())
}

/// Show details about a specific tag
///
/// # Errors
///
/// Returns an error if:
/// - The repository is not initialized
/// - The tag does not exist
/// - Failed to read tag reference
/// - Failed to load commit details
pub fn show(ctx: &DotmanContext, name: &str) -> Result<()> {
    ctx.check_repo_initialized()?;

    let ref_manager = RefManager::new(ctx.repo_path.clone());

    if !ref_manager.tag_exists(name) {
        return Err(anyhow::anyhow!("Tag '{}' does not exist", name));
    }

    let commit_id = ref_manager.get_tag_commit(name)?;

    // Load the snapshot to get commit details
    let snapshot_manager =
        SnapshotManager::new(ctx.repo_path.clone(), ctx.config.core.compression_level);
    let snapshot = snapshot_manager
        .load_snapshot(&commit_id)
        .with_context(|| format!("Failed to load commit details for tag '{name}'"))?;

    let commit = &snapshot.commit;

    // Display tag and commit details
    println!("{} {}", "Tag:".bold(), name.yellow());
    println!("{} {}", "Commit:".bold(), commit_id.yellow());

    if let Some(parent) = &commit.parent {
        let parent_display = if parent.len() >= 8 {
            &parent[..8]
        } else {
            parent
        };
        println!("{} {}", "Parent:".bold(), parent_display);
    }

    println!("{} {}", "Author:".bold(), commit.author);

    // Format timestamp
    let datetime = Local
        .timestamp_opt(commit.timestamp, 0)
        .single()
        .unwrap_or_else(Local::now);
    println!(
        "{} {}",
        "Date:".bold(),
        datetime.format("%Y-%m-%d %H:%M:%S")
    );

    println!("\n{}", "Message:".bold());
    println!("    {}", commit.message);

    // Show file count
    println!("\n{} {} files", "Files:".bold(), snapshot.files.len());

    Ok(())
}
