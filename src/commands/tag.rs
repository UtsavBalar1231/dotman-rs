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
                let message_preview = snapshot_manager
                    .load_snapshot(&commit_id)
                    .ok()
                    .map_or_else(
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn setup_test_context() -> Result<(tempfile::TempDir, DotmanContext)> {
        use crate::test_utils::fixtures::{create_test_context, test_commit_id};
        use crate::storage::{Commit, snapshots::Snapshot};
        use std::collections::HashMap;

        let (dir, ctx) = create_test_context()?;

        // Create a test commit that actually exists
        let commit_id = test_commit_id("abc123def456");
        let commit = Commit {
            id: commit_id.clone(),
            parent: None,
            message: "Test commit".to_string(),
            author: "Test User".to_string(),
            timestamp: 1_234_567_890,
            tree_hash: "test_tree".to_string(),
        };

        let snapshot = Snapshot {
            commit,
            files: HashMap::new(),
        };

        // Save the snapshot
        let serialized = crate::utils::serialization::serialize(&snapshot)?;
        let compressed = zstd::stream::encode_all(&serialized[..], 3)?;
        let snapshot_path = ctx.repo_path.join("commits").join(format!("{}.zst", &commit_id));
        fs::write(&snapshot_path, compressed)?;

        // Update HEAD to point to this commit
        let head_path = ctx.repo_path.join("refs/heads/main");
        fs::write(&head_path, &commit_id)?;

        Ok((dir, ctx))
    }

    #[test]
    fn test_create_tag() -> Result<()> {
        let (_dir, ctx) = setup_test_context()?;

        create(&ctx, "v1.0.0", None)?;

        // Verify tag was created
        let ref_manager = RefManager::new(ctx.repo_path);
        assert!(ref_manager.tag_exists("v1.0.0"));

        Ok(())
    }

    #[test]
    fn test_create_tag_with_invalid_commit() -> Result<()> {
        let (_dir, ctx) = setup_test_context()?;

        // Try to create tag with non-existent commit
        let result = create(&ctx, "v1.0.0", Some("nonexistent"));
        assert!(result.is_err());
        // The error message varies based on whether it's treated as a ref or commit hash
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("does not exist") || 
            err_msg.contains("Cannot resolve") ||
            err_msg.contains("Failed to resolve"),
            "Unexpected error message: {err_msg}"
        );

        Ok(())
    }

    #[test]
    fn test_create_tag_with_empty_repo() -> Result<()> {
        let (_dir, ctx) = setup_test_context()?;

        // Clear the HEAD reference to simulate empty repo
        let head_path = ctx.repo_path.join("refs/heads/main");
        fs::remove_file(&head_path)?;

        // Try to create tag without specifying commit (should fail)
        let result = create(&ctx, "v1.0.0", None);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("No commits available")
        );

        Ok(())
    }

    #[test]
    fn test_list_tags_empty() -> Result<()> {
        let (_dir, ctx) = setup_test_context()?;

        // List tags when none exist
        let result = list(&ctx);
        assert!(result.is_ok());

        Ok(())
    }

    #[test]
    fn test_list_tags() -> Result<()> {
        let (_dir, ctx) = setup_test_context()?;

        // Create some tags
        let ref_manager = RefManager::new(ctx.repo_path.clone());
        ref_manager.create_tag("v1.0.0", Some("abc123"))?;
        ref_manager.create_tag("v2.0.0", Some("def456"))?;

        // List should succeed
        let result = list(&ctx);
        assert!(result.is_ok());

        Ok(())
    }

    #[test]
    fn test_delete_tag() -> Result<()> {
        let (_dir, ctx) = setup_test_context()?;

        // Create and then delete a tag
        let ref_manager = RefManager::new(ctx.repo_path.clone());
        ref_manager.create_tag("temp", None)?;
        assert!(ref_manager.tag_exists("temp"));

        delete(&ctx, "temp", false)?;
        assert!(!ref_manager.tag_exists("temp"));

        Ok(())
    }

    #[test]
    fn test_delete_nonexistent_tag() -> Result<()> {
        let (_dir, ctx) = setup_test_context()?;

        // Try to delete a tag that doesn't exist
        let result = delete(&ctx, "nonexistent", false);
        assert!(result.is_err());

        Ok(())
    }

    #[test]
    fn test_show_tag() -> Result<()> {
        let (_dir, ctx) = setup_test_context()?;

        // Create tag using our validated create function
        create(&ctx, "v1.0.0", None)?;

        let result = show(&ctx, "v1.0.0");
        assert!(result.is_ok());

        Ok(())
    }

    #[test]
    fn test_show_nonexistent_tag() -> Result<()> {
        let (_dir, ctx) = setup_test_context()?;

        // Try to show a tag that doesn't exist
        let result = show(&ctx, "nonexistent");
        assert!(result.is_err());

        Ok(())
    }

    #[test]
    fn test_duplicate_tag() -> Result<()> {
        let (_dir, ctx) = setup_test_context()?;

        create(&ctx, "v1.0.0", None)?;

        // Try to create the same tag again
        let result = create(&ctx, "v1.0.0", None);
        assert!(result.is_err());

        Ok(())
    }

    #[test]
    fn test_invalid_tag_names() -> Result<()> {
        let (_dir, ctx) = setup_test_context()?;

        // Test various invalid tag names
        let invalid_names = vec![
            "",                     // Empty
            ".",                    // Reserved
            "..",                   // Reserved
            "HEAD",                 // Reserved
            "tag/with/slash",       // Contains slash
            "tag\\with\\backslash", // Contains backslash
            ".hidden",              // Starts with dot
            "trailing.",            // Ends with dot
            " spaces ",             // Starts/ends with space
        ];

        for invalid_name in invalid_names {
            let result = create(&ctx, invalid_name, None);
            assert!(
                result.is_err(),
                "Tag name '{invalid_name}' should be invalid"
            );
        }

        Ok(())
    }
}
