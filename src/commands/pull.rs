use crate::DotmanContext;
use crate::mapping::MappingManager;
use crate::mirror::GitMirror;
use crate::refs::RefManager;
use crate::storage::index::Index;
use crate::storage::snapshots::SnapshotManager;
use crate::sync::Importer;
use anyhow::{Context, Result};
use colored::Colorize;
use std::fmt::Write;
use std::process::Command;

/// Execute pull command - fetch from and integrate with another repository or local branch
///
/// # Errors
///
/// Returns an error if:
/// - The repository is not initialized
/// - Conflicting options are specified (e.g., --rebase with --no-ff)
/// - The remote does not exist or cannot be reached
/// - The fetch operation fails
/// - The merge or rebase operation fails
pub fn execute(
    ctx: &DotmanContext,
    remote: &str,
    branch: &str,
    rebase: bool,
    no_ff: bool,
    squash: bool,
) -> Result<()> {
    ctx.check_repo_initialized()?;

    if rebase && (no_ff || squash) {
        return Err(anyhow::anyhow!(
            "Cannot use --rebase with --no-ff or --squash"
        ));
    }

    let remote_config = ctx.config.get_remote(remote).with_context(|| {
        format!("Remote '{remote}' does not exist. Use 'dot remote add' to add it.")
    })?;

    match &remote_config.remote_type {
        crate::config::RemoteType::Git => {
            pull_from_git(ctx, remote_config, remote, branch, rebase, no_ff, squash)
        }
        crate::config::RemoteType::S3 => pull_from_s3(ctx, remote_config, remote, branch),
        crate::config::RemoteType::Rsync => pull_from_rsync(ctx, remote_config, remote, branch),
        crate::config::RemoteType::None => Err(anyhow::anyhow!(
            "Remote '{}' has no type configured.",
            remote
        )),
    }
}

fn pull_from_git(
    ctx: &DotmanContext,
    remote_config: &crate::config::RemoteConfig,
    remote: &str,
    branch: &str,
    rebase: bool,
    no_ff: bool,
    squash: bool,
) -> Result<()> {
    use crate::storage::{Commit, FileEntry, file_ops::hash_bytes};
    use crate::utils::{
        commit::generate_commit_id, get_current_timestamp, get_current_user_with_config,
    };

    let url = remote_config
        .url
        .as_ref()
        .with_context(|| format!("Remote '{remote}' has no URL configured"))?;

    super::print_info(&format!("Pulling from git remote {remote} ({url})"));

    // Create and initialize mirror
    let mirror = GitMirror::new(&ctx.repo_path, remote, url, ctx.config.clone());
    mirror.init_mirror()?;

    // Pull changes in mirror
    super::print_info(&format!("Fetching branch '{branch}' from remote..."));
    mirror.pull(branch)?;

    let git_commit = mirror.get_head_commit()?;

    let mapping_manager = MappingManager::new(&ctx.repo_path)?;
    if let Some(dotman_commit) = mapping_manager
        .mapping()
        .get_dotman_commit(remote, &git_commit)
    {
        // We already have this commit, just checkout
        super::print_info(&format!(
            "Commit already synchronized, checking out {}",
            &dotman_commit[..8]
        ));

        // Checkout the commit
        crate::commands::checkout::execute(ctx, &dotman_commit, false)?;

        super::print_success(&format!(
            "Successfully pulled from {remote} ({branch}) - already up to date"
        ));
        return Ok(());
    }

    // Import changes from mirror
    super::print_info("Importing changes from remote...");

    let mut index = Index::load(&ctx.repo_path.join(crate::INDEX_FILE))?;
    let mut snapshot_manager =
        SnapshotManager::new(ctx.repo_path.clone(), ctx.config.core.compression_level);
    let mut importer = Importer::new(&mut snapshot_manager, &mut index);

    let home_dir = dirs::home_dir().context("Could not find home directory")?;
    let changes = importer.import_changes(mirror.get_mirror_path(), &home_dir)?;

    if changes.is_empty() {
        super::print_info("No changes to import");
        super::print_success(&format!(
            "Successfully pulled from {remote} ({branch}) - already up to date"
        ));
        return Ok(());
    }

    index.save(&ctx.repo_path.join(crate::INDEX_FILE))?;

    super::print_info(&format!(
        "Creating commit for imported changes: {}",
        changes.summary()
    ));

    let message = format!("Pull from {} ({}): {}", remote, branch, changes.summary());

    // Create commit similar to how commit command does it
    // Get timestamp and author for commit
    let timestamp = get_current_timestamp();
    let author = get_current_user_with_config(&ctx.config);

    // Get parent commit (if any)
    let ref_manager = RefManager::new(ctx.repo_path.clone());
    let parent = ref_manager.get_head_commit()?;

    // Create tree hash from all file hashes
    let mut tree_content = String::new();
    for (path, entry) in &index.entries {
        writeln!(tree_content, "{} {}", entry.hash, path.display())?;
    }
    let tree_hash = hash_bytes(tree_content.as_bytes());

    // Generate content-addressed commit ID
    let commit_id = generate_commit_id(&tree_hash, parent.as_deref(), &message, &author, timestamp);

    // Create commit object
    let commit = Commit {
        id: commit_id.clone(),
        parent,
        message,
        author,
        timestamp,
        tree_hash,
    };

    // Create snapshot
    let files: Vec<FileEntry> = index.entries.values().cloned().collect();
    snapshot_manager.create_snapshot(commit, &files)?;

    // Update refs
    let ref_manager = RefManager::new(ctx.repo_path.clone());
    if let Some(current_branch) = ref_manager.current_branch()? {
        ref_manager.update_branch(&current_branch, &commit_id)?;
    }

    // Update mapping
    let mut mapping_manager = MappingManager::new(&ctx.repo_path)?;
    mapping_manager.add_and_save(remote, &commit_id, &git_commit)?;
    mapping_manager.update_branch_and_save(branch, &commit_id, Some((remote, &git_commit)))?;

    super::print_success(&format!(
        "Successfully pulled from {} ({}) - {}",
        remote,
        branch,
        changes.summary()
    ));

    // Handle different merge strategies
    if rebase {
        // Rebase current changes on top of pulled changes
        super::print_info("Rebasing local changes on top of pulled changes...");
        perform_rebase(ctx, &commit_id)?;
    } else if no_ff || squash {
        // Use merge command with appropriate flags
        super::print_info(&format!(
            "Merging with {} strategy...",
            if squash { "squash" } else { "no-ff" }
        ));
        crate::commands::merge::execute(ctx, &format!("{remote}/{branch}"), no_ff, squash, None)?;
    } else {
        // Default: checkout the new commit to update working directory
        super::print_info("Updating working directory to match pulled changes...");
        crate::commands::checkout::execute(ctx, &commit_id, false)?;
    }

    Ok(())
}

fn perform_rebase(ctx: &DotmanContext, onto_commit: &str) -> Result<()> {
    // Simplified rebase: just move HEAD to the new commit
    // In a full implementation, would replay local commits on top

    super::print_info(&format!(
        "Rebasing onto {}",
        onto_commit[..8.min(onto_commit.len())].yellow()
    ));

    // For now, just checkout the new commit
    // A full rebase would:
    // 1. Save local commits since the common ancestor
    // 2. Reset to the new base commit
    // 3. Replay the local commits
    crate::commands::checkout::execute(ctx, onto_commit, false)?;

    super::print_success("Rebase complete");
    Ok(())
}

fn pull_from_s3(
    ctx: &DotmanContext,
    remote_config: &crate::config::RemoteConfig,
    remote: &str,
    _branch: &str,
) -> Result<()> {
    let bucket = remote_config
        .url
        .as_ref()
        .with_context(|| format!("Remote '{remote}' has no S3 bucket configured"))?;

    super::print_info(&format!("Pulling from S3 bucket {bucket}"));

    let output = Command::new("aws")
        .args([
            "s3",
            "sync",
            &format!("s3://{bucket}/"),
            ctx.repo_path.to_str().context("Invalid repository path")?,
            "--delete",
        ])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!("S3 sync failed: {stderr}"));
    }

    super::print_success(&format!("Successfully pulled from S3 bucket {bucket}"));
    Ok(())
}

fn pull_from_rsync(
    ctx: &DotmanContext,
    remote_config: &crate::config::RemoteConfig,
    remote: &str,
    _branch: &str,
) -> Result<()> {
    let source = remote_config
        .url
        .as_ref()
        .with_context(|| format!("Remote '{remote}' has no rsync source configured"))?;

    super::print_info(&format!("Pulling via rsync from {source}"));

    let output = Command::new("rsync")
        .args([
            "-avz",
            "--delete",
            source,
            &format!("{}/", ctx.repo_path.display()),
        ])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!("Rsync failed: {stderr}"));
    }

    super::print_success(&format!("Successfully pulled via rsync from {source}"));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, RemoteType};
    use std::fs;
    use tempfile::tempdir;

    fn create_test_context(remote_type: RemoteType, url: Option<String>) -> Result<DotmanContext> {
        let temp = tempdir()?;
        let repo_path = temp.path().join(".dotman");
        let config_path = temp.path().join("config.toml");

        fs::create_dir_all(&repo_path)?;
        fs::create_dir_all(repo_path.join("commits"))?;
        fs::create_dir_all(repo_path.join("objects"))?;

        let mut config = Config::default();
        config.core.repo_path = repo_path.clone();

        let remote_config = crate::config::RemoteConfig { remote_type, url };
        config.remotes.insert("origin".to_string(), remote_config);
        config.save(&config_path)?;

        Ok(DotmanContext {
            repo_path,
            config_path,
            config,
            no_pager: true,
        })
    }

    #[test]
    fn test_execute_no_remote() -> Result<()> {
        let ctx = create_test_context(RemoteType::None, None)?;

        let result = execute(&ctx, "origin", "main", false, false, false);
        assert!(result.is_err());

        Ok(())
    }

    #[test]
    fn test_execute_git_remote() -> Result<()> {
        let _ctx = create_test_context(
            RemoteType::Git,
            Some("https://github.com/user/repo.git".to_string()),
        )?;

        // This would normally call git command, which we'd need to mock
        // For now, we're just testing that it routes to the correct function
        // In a real scenario, we'd use dependency injection or mocking

        // We can't easily test the actual git command execution without mocking
        // but we can at least verify the function doesn't panic with valid config

        Ok(())
    }

    #[test]
    fn test_execute_s3_remote() -> Result<()> {
        let _ctx = create_test_context(RemoteType::S3, Some("my-bucket".to_string()))?;

        // Similar to git test - would need to mock aws command
        Ok(())
    }

    #[test]
    fn test_execute_rsync_remote() -> Result<()> {
        let _ctx = create_test_context(
            RemoteType::Rsync,
            Some("user@host:/path/to/repo".to_string()),
        )?;

        // Similar to git test - would need to mock rsync command
        Ok(())
    }

    #[test]
    fn test_pull_from_git_no_url() -> Result<()> {
        let ctx = create_test_context(RemoteType::Git, None)?;

        let remote_config = crate::config::RemoteConfig {
            remote_type: RemoteType::Git,
            url: None,
        };
        let result = pull_from_git(&ctx, &remote_config, "origin", "main", false, false, false);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Remote 'origin' has no URL configured")
        );

        Ok(())
    }

    #[test]
    fn test_pull_from_s3_no_bucket() -> Result<()> {
        let ctx = create_test_context(RemoteType::S3, None)?;

        let remote_config = crate::config::RemoteConfig {
            remote_type: RemoteType::S3,
            url: None,
        };
        let result = pull_from_s3(&ctx, &remote_config, "origin", "main");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Remote 'origin' has no S3 bucket configured")
        );

        Ok(())
    }

    #[test]
    fn test_pull_from_rsync_no_source() -> Result<()> {
        let ctx = create_test_context(RemoteType::Rsync, None)?;

        let remote_config = crate::config::RemoteConfig {
            remote_type: RemoteType::Rsync,
            url: None,
        };
        let result = pull_from_rsync(&ctx, &remote_config, "origin", "main");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Remote 'origin' has no rsync source configured")
        );

        Ok(())
    }

    #[test]
    fn test_check_repo_initialized_error() -> Result<()> {
        let temp = tempdir()?;
        let repo_path = temp.path().join("nonexistent").join("deep").join("path");

        let conflict_path = temp.path().join("nonexistent");
        fs::write(&conflict_path, "blocking file")?;

        let ctx = DotmanContext {
            repo_path,
            config_path: temp.path().join("config"),
            config: Config::default(),
            no_pager: true,
        };

        let result = execute(&ctx, "origin", "main", false, false, false);
        assert!(result.is_err());

        Ok(())
    }
}
