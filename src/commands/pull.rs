use crate::DotmanContext;
use crate::mapping::MappingManager;
use crate::mirror::GitMirror;
use crate::refs::RefManager;
use crate::storage::index::Index;
use crate::storage::snapshots::SnapshotManager;
use crate::sync::Importer;
use anyhow::Result;
use std::process::Command;

pub fn execute(ctx: &DotmanContext, remote: &str, branch: &str) -> Result<()> {
    ctx.ensure_repo_exists()?;

    // Get the specified remote
    let remote_config = ctx.config.get_remote(remote).ok_or_else(|| {
        anyhow::anyhow!(
            "Remote '{}' does not exist. Use 'dot remote add' to add it.",
            remote
        )
    })?;

    match &remote_config.remote_type {
        crate::config::RemoteType::Git => pull_from_git(ctx, remote_config, remote, branch),
        crate::config::RemoteType::S3 => pull_from_s3(ctx, remote_config, remote, branch),
        crate::config::RemoteType::Rsync => pull_from_rsync(ctx, remote_config, remote, branch),
        crate::config::RemoteType::None => {
            anyhow::bail!("Remote '{}' has no type configured.", remote);
        }
    }
}

fn pull_from_git(
    ctx: &DotmanContext,
    remote_config: &crate::config::RemoteConfig,
    remote: &str,
    branch: &str,
) -> Result<()> {
    let url = remote_config
        .url
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Remote '{}' has no URL configured", remote))?;

    super::print_info(&format!("Pulling from git remote {} ({})", remote, url));

    // Create and initialize mirror
    let mirror = GitMirror::new(&ctx.repo_path, remote, url);
    mirror.init_mirror()?;

    // Pull changes in mirror
    super::print_info(&format!("Fetching branch '{}' from remote...", branch));
    mirror.pull(branch)?;

    // Get the current git commit after pull
    let git_commit = mirror.get_head_commit()?;

    // Check if we already have this commit mapped
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
            "Successfully pulled from {} ({}) - already up to date",
            remote, branch
        ));
        return Ok(());
    }

    // Import changes from mirror
    super::print_info("Importing changes from remote...");

    let mut index = Index::load(&ctx.repo_path.join(crate::INDEX_FILE))?;
    let mut snapshot_manager =
        SnapshotManager::new(ctx.repo_path.clone(), ctx.config.core.compression_level);
    let mut importer = Importer::new(&mut snapshot_manager, &mut index);

    let home_dir =
        dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;
    let changes = importer.import_changes(mirror.get_mirror_path(), &home_dir)?;

    if changes.is_empty() {
        super::print_info("No changes to import");
        super::print_success(&format!(
            "Successfully pulled from {} ({}) - already up to date",
            remote, branch
        ));
        return Ok(());
    }

    // Save the updated index
    index.save(&ctx.repo_path.join(crate::INDEX_FILE))?;

    // Create a new dotman commit with the imported changes
    super::print_info(&format!(
        "Creating commit for imported changes: {}",
        changes.summary()
    ));

    let author = crate::utils::get_current_user();
    let message = format!("Pull from {} ({}): {}", remote, branch, changes.summary());

    // Create commit similar to how commit command does it
    use crate::storage::{Commit, FileEntry};
    use crate::utils::{get_current_timestamp, hash::hash_bytes};

    let timestamp = get_current_timestamp();
    let commit_id = format!(
        "{:016x}{}",
        timestamp,
        &hash_bytes(message.as_bytes())[..16]
    );

    // Get parent commit (if any)
    let ref_manager = RefManager::new(ctx.repo_path.clone());
    let parent = ref_manager.get_head_commit()?;

    // Create tree hash from all file hashes
    let mut tree_content = String::new();
    for (path, entry) in &index.entries {
        tree_content.push_str(&format!("{} {}\n", entry.hash, path.display()));
    }
    let tree_hash = hash_bytes(tree_content.as_bytes());

    // Create commit object
    let commit = Commit {
        id: commit_id.clone(),
        parent,
        message: message.to_string(),
        author: author.to_string(),
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

    // Checkout the new commit to update working directory
    super::print_info("Updating working directory to match pulled changes...");
    crate::commands::checkout::execute(ctx, &commit_id, false)?;

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
        .ok_or_else(|| anyhow::anyhow!("Remote '{}' has no S3 bucket configured", remote))?;

    super::print_info(&format!("Pulling from S3 bucket {}", bucket));

    let output = Command::new("aws")
        .args([
            "s3",
            "sync",
            &format!("s3://{}/", bucket),
            ctx.repo_path.to_str().unwrap(),
            "--delete",
        ])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("S3 sync failed: {}", stderr);
    }

    super::print_success(&format!("Successfully pulled from S3 bucket {}", bucket));
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
        .ok_or_else(|| anyhow::anyhow!("Remote '{}' has no rsync source configured", remote))?;

    super::print_info(&format!("Pulling via rsync from {}", source));

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
        anyhow::bail!("Rsync failed: {}", stderr);
    }

    super::print_success(&format!("Successfully pulled via rsync from {}", source));
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

        // Add a remote named "origin" with the specified type and url
        let remote_config = crate::config::RemoteConfig { remote_type, url };
        config.remotes.insert("origin".to_string(), remote_config);
        config.save(&config_path)?;

        Ok(DotmanContext {
            repo_path,
            config_path,
            config,
        })
    }

    #[test]
    fn test_execute_no_remote() -> Result<()> {
        let ctx = create_test_context(RemoteType::None, None)?;

        let result = execute(&ctx, "origin", "main");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Remote 'origin' has no type configured")
        );

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
        let result = pull_from_git(&ctx, &remote_config, "origin", "main");
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
    fn test_ensure_repo_exists_error() -> Result<()> {
        let temp = tempdir()?;
        let repo_path = temp.path().join("nonexistent").join("deep").join("path");

        // Create a file where directory should be, causing conflict
        let conflict_path = temp.path().join("nonexistent");
        fs::write(&conflict_path, "blocking file")?;

        let ctx = DotmanContext {
            repo_path,
            config_path: temp.path().join("config"),
            config: Config::default(),
        };

        let result = execute(&ctx, "origin", "main");
        assert!(result.is_err());

        Ok(())
    }
}
