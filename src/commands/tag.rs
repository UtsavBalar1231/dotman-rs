use crate::DotmanContext;
use crate::refs::RefManager;
use anyhow::Result;
use colored::Colorize;

/// Create a new tag
pub fn create(ctx: &DotmanContext, name: &str, commit: Option<&str>) -> Result<()> {
    ctx.check_repo_initialized()?;

    let ref_manager = RefManager::new(ctx.repo_path.clone());

    // Validate tag name (basic validation)
    if name.is_empty() {
        anyhow::bail!("Tag name cannot be empty");
    }

    // If commit is provided, validate it exists
    // TODO: Add proper commit validation once we have commit lookup functionality

    ref_manager.create_tag(name, commit)?;

    let target = if let Some(c) = commit {
        &c[..8.min(c.len())]
    } else {
        "HEAD"
    };

    super::print_success(&format!("Created tag '{}' at {}", name, target));
    Ok(())
}

/// List all tags
pub fn list(ctx: &DotmanContext) -> Result<()> {
    ctx.check_repo_initialized()?;

    let ref_manager = RefManager::new(ctx.repo_path.clone());
    let tags = ref_manager.list_tags()?;

    if tags.is_empty() {
        super::print_info("No tags exist");
        return Ok(());
    }

    println!("{}", "Tags:".bold());
    for tag in tags {
        // Get the commit for each tag
        match ref_manager.get_tag_commit(&tag) {
            Ok(commit_id) => {
                let short_commit = &commit_id[..8.min(commit_id.len())];
                println!("  {} -> {}", tag.yellow(), short_commit.dimmed());
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
pub fn delete(ctx: &DotmanContext, name: &str, force: bool) -> Result<()> {
    ctx.check_repo_initialized()?;

    let ref_manager = RefManager::new(ctx.repo_path.clone());

    if !ref_manager.tag_exists(name) {
        anyhow::bail!("Tag '{}' does not exist", name);
    }

    // In a real implementation, we might want to check if the tag
    // points to an important commit and require force flag
    if !force {
        // For now, just show a warning
        super::print_warning(&format!("Deleting tag '{}'", name));
    }

    ref_manager.delete_tag(name)?;
    super::print_success(&format!("Deleted tag '{}'", name));

    Ok(())
}

/// Show details about a specific tag
pub fn show(ctx: &DotmanContext, name: &str) -> Result<()> {
    ctx.check_repo_initialized()?;

    let ref_manager = RefManager::new(ctx.repo_path.clone());

    if !ref_manager.tag_exists(name) {
        anyhow::bail!("Tag '{}' does not exist", name);
    }

    let commit_id = ref_manager.get_tag_commit(name)?;

    println!("{} {}", "Tag:".bold(), name.yellow());
    println!("{} {}", "Commit:".bold(), commit_id);

    // TODO: Once we have commit message storage, show the commit message here
    // For now, just show the commit ID

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn setup_test_context() -> Result<(tempfile::TempDir, DotmanContext)> {
        use crate::test_utils::fixtures::create_test_context;

        let (dir, ctx) = create_test_context()?;

        // Create a dummy commit for HEAD
        let head_path = ctx.repo_path.join("refs/heads/main");
        fs::write(&head_path, "abc123def456")?;

        Ok((dir, ctx))
    }

    #[test]
    fn test_create_tag() -> Result<()> {
        let (_dir, ctx) = setup_test_context()?;

        // Create a tag
        create(&ctx, "v1.0.0", None)?;

        // Verify tag was created
        let ref_manager = RefManager::new(ctx.repo_path.clone());
        assert!(ref_manager.tag_exists("v1.0.0"));

        Ok(())
    }

    #[test]
    fn test_create_tag_with_commit() -> Result<()> {
        let (_dir, ctx) = setup_test_context()?;

        // Create a tag pointing to a specific commit
        create(&ctx, "v1.0.0", Some("fedcba987654"))?;

        // Verify tag points to correct commit
        let ref_manager = RefManager::new(ctx.repo_path.clone());
        let commit = ref_manager.get_tag_commit("v1.0.0")?;
        assert_eq!(commit, "fedcba987654");

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

        // Create a tag and show it
        let ref_manager = RefManager::new(ctx.repo_path.clone());
        ref_manager.create_tag("v1.0.0", Some("abc123def456"))?;

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

        // Create a tag
        create(&ctx, "v1.0.0", None)?;

        // Try to create the same tag again
        let result = create(&ctx, "v1.0.0", None);
        assert!(result.is_err());

        Ok(())
    }
}
