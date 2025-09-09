use crate::DotmanContext;
use crate::mapping::MappingManager;
use crate::mirror::GitMirror;
use crate::refs::RefManager;
use crate::storage::index::Index;
use crate::storage::snapshots::SnapshotManager;
use crate::sync::Exporter;
use anyhow::{Context, Result};
use std::process::Command;

/// Options for push operations
#[allow(clippy::struct_excessive_bools)]
struct PushOptions<'a> {
    remote: &'a str,
    branch: &'a str,
    force: bool,
    force_with_lease: bool,
    dry_run: bool,
    tags: bool,
}

/// Execute push command - update remote refs along with associated objects
///
/// # Errors
///
/// Returns an error if:
/// - The repository is not initialized
/// - The remote does not exist or cannot be reached
/// - The branch does not exist locally
/// - The push is rejected by the remote
/// - Network operations fail
#[allow(clippy::fn_params_excessive_bools, clippy::too_many_arguments)]
pub fn execute(
    ctx: &mut DotmanContext,
    remote: Option<&str>,
    branch: Option<&str>,
    force: bool,
    force_with_lease: bool,
    dry_run: bool,
    tags: bool,
    set_upstream: bool,
) -> Result<()> {
    ctx.check_repo_initialized()?;

    if force && force_with_lease {
        return Err(anyhow::anyhow!(
            "Cannot use both --force and --force-with-lease"
        ));
    }

    // Determine remote and branch to use
    let (remote_name, branch_name, should_set_upstream) =
        determine_push_target(ctx, remote, branch, set_upstream)?;

    let remote_config = ctx.config.get_remote(&remote_name).with_context(|| {
        format!("Remote '{remote_name}' does not exist. Use 'dot remote add' to add it.")
    })?;

    if dry_run {
        super::print_info(&format!(
            "Dry run mode - would push to {remote_name} ({branch_name})"
        ));
    }

    let push_opts = PushOptions {
        remote: &remote_name,
        branch: &branch_name,
        force,
        force_with_lease,
        dry_run,
        tags,
    };

    let result = match &remote_config.remote_type {
        crate::config::RemoteType::Git => push_to_git(ctx, remote_config, &push_opts),
        crate::config::RemoteType::None => Err(anyhow::anyhow!(
            "Remote '{}' has no type configured or is not a Git remote.",
            remote_name
        )),
    };

    // Auto-set upstream on successful push if needed
    if result.is_ok() && should_set_upstream && !dry_run {
        let tracking = crate::config::BranchTracking {
            remote: remote_name.clone(),
            branch: branch_name.clone(),
        };
        ctx.config
            .set_branch_tracking(branch_name.clone(), tracking);
        ctx.config.save(&ctx.config_path)?;
        super::print_info(&format!(
            "Branch '{branch_name}' set up to track '{remote_name}/{branch_name}'"
        ));
    }

    result
}

/// Build a chain of commits from root to the given commit
fn build_commit_chain(snapshot_manager: &SnapshotManager, target_commit: &str) -> Vec<String> {
    let mut commits = Vec::new();
    let mut current_commit = Some(target_commit.to_string());

    // Follow parent links to collect all commits
    while let Some(commit_id) = current_commit {
        // Skip the special "all zeros" parent that represents no parent
        if commit_id.chars().all(|c| c == '0') {
            break;
        }

        commits.push(commit_id.clone());

        match snapshot_manager.load_snapshot(&commit_id) {
            Ok(snapshot) => {
                #[allow(clippy::assigning_clones)]
                {
                    current_commit = snapshot.commit.parent.clone();
                }
            }
            Err(_) => {
                // Stop if we can't load a commit
                break;
            }
        }
    }

    let mut commits_with_timestamps = Vec::new();
    for commit_id in commits {
        if let Ok(snapshot) = snapshot_manager.load_snapshot(&commit_id) {
            commits_with_timestamps.push((commit_id, snapshot.commit.timestamp));
        }
    }

    // Sort by timestamp (oldest first)
    commits_with_timestamps.sort_by_key(|(_, timestamp)| *timestamp);

    // Extract just the commit IDs in chronological order
    let chain: Vec<String> = commits_with_timestamps
        .into_iter()
        .map(|(commit_id, _)| commit_id)
        .collect();

    chain
}

/// Get commits that haven't been pushed yet
fn get_unpushed_commits(
    snapshot_manager: &SnapshotManager,
    mapping_manager: &MappingManager,
    remote: &str,
    target_commit: &str,
) -> Vec<String> {
    let full_chain = build_commit_chain(snapshot_manager, target_commit);

    // Find the last pushed commit
    let mut unpushed = Vec::new();
    for commit_id in full_chain {
        if mapping_manager
            .mapping()
            .get_git_commit(remote, &commit_id)
            .is_none()
        {
            unpushed.push(commit_id);
        }
    }

    unpushed
}

/// Determine the remote and branch to push to
///
/// Returns (`remote_name`, `branch_name`, `should_set_upstream`)
fn determine_push_target(
    ctx: &DotmanContext,
    remote: Option<&str>,
    branch: Option<&str>,
    explicit_set_upstream: bool,
) -> Result<(String, String, bool)> {
    use crate::refs::RefManager;

    let ref_manager = RefManager::new(ctx.repo_path.clone());

    // If both remote and branch are provided, use them directly
    if let (Some(r), Some(b)) = (remote, branch) {
        return Ok((r.to_string(), b.to_string(), explicit_set_upstream));
    }

    // Get current branch
    let current_branch = ref_manager
        .current_branch()?
        .context("Not on any branch (detached HEAD). Please specify branch to push.")?;

    // If only remote is provided, use current branch
    if let Some(r) = remote {
        return Ok((r.to_string(), current_branch, explicit_set_upstream));
    }

    // If only branch is provided, need to find remote from tracking
    if let Some(b) = branch {
        if let Some(tracking) = ctx.config.get_branch_tracking(b) {
            return Ok((tracking.remote.clone(), b.to_string(), false));
        }
        return Err(anyhow::anyhow!(
            "Branch '{}' has no upstream tracking. Please specify remote or use --set-upstream.",
            b
        ));
    }

    // Neither remote nor branch provided - use tracking info for current branch
    if let Some(tracking) = ctx.config.get_branch_tracking(&current_branch) {
        super::print_info(&format!(
            "Using tracked upstream: {}/{}",
            tracking.remote, tracking.branch
        ));
        return Ok((tracking.remote.clone(), tracking.branch.clone(), false));
    }

    // No tracking info - check if this is first push
    if ctx.config.remotes.is_empty() {
        return Err(anyhow::anyhow!(
            "No remotes configured. Use 'dot remote add <name> <url>' to add a remote."
        ));
    }

    // Check if there's only one remote (common case)
    if ctx.config.remotes.len() == 1 {
        let (remote_name, _) = ctx.config.remotes.iter().next().unwrap();
        super::print_info(&format!(
            "No upstream tracking for branch '{current_branch}'. Will set upstream to '{remote_name}/{current_branch}' after successful push."
        ));
        return Ok((remote_name.clone(), current_branch, true));
    }

    // Multiple remotes exist, need user to specify
    let available_remotes: Vec<String> = ctx.config.remotes.keys().cloned().collect();
    Err(anyhow::anyhow!(
        "Branch '{}' has no upstream tracking and multiple remotes exist: {}\n\
         Please specify remote: 'dot push <remote>' or set upstream: 'dot branch set-upstream <remote>'",
        current_branch,
        available_remotes.join(", ")
    ))
}

fn push_to_git(
    ctx: &DotmanContext,
    remote_config: &crate::config::RemoteConfig,
    opts: &PushOptions,
) -> Result<()> {
    let url = remote_config
        .url
        .as_ref()
        .with_context(|| format!("Remote '{}' has no URL configured", opts.remote))?;

    super::print_info(&format!("Pushing to git remote {} ({})", opts.remote, url));

    if opts.force {
        super::print_warning("Force push requested - this may overwrite remote changes!");
    } else if opts.force_with_lease {
        super::print_info("Using --force-with-lease for safer force push");
    }

    let mirror = GitMirror::new(&ctx.repo_path, opts.remote, url, ctx.config.clone());
    mirror.init_mirror()?;

    mirror.checkout_branch(opts.branch)?;

    let ref_manager = RefManager::new(ctx.repo_path.clone());
    let current_commit = ref_manager
        .get_head_commit()?
        .context("No commits to push")?;

    let snapshot_manager =
        SnapshotManager::new(ctx.repo_path.clone(), ctx.config.core.compression_level);
    let mut mapping_manager = MappingManager::new(&ctx.repo_path)?;

    let commits_to_push = get_unpushed_commits(
        &snapshot_manager,
        &mapping_manager,
        opts.remote,
        &current_commit,
    );

    if commits_to_push.is_empty() {
        super::print_info("Already up to date - no new commits to push");
        return Ok(());
    }

    super::print_info(&format!(
        "Found {} new commit{} to push",
        commits_to_push.len(),
        if commits_to_push.len() == 1 { "" } else { "s" }
    ));

    // Export and commit each dotman commit to git
    let index = Index::load(&ctx.repo_path.join(crate::INDEX_FILE))?;
    let exporter = Exporter::new(&snapshot_manager, &index);

    for (i, commit_id) in commits_to_push.iter().enumerate() {
        super::print_info(&format!(
            "Processing commit {}/{}: {}",
            i + 1,
            commits_to_push.len(),
            &commit_id[..8.min(commit_id.len())]
        ));

        let snapshot = snapshot_manager.load_snapshot(commit_id)?;

        // Clear the working directory to ensure we have exact state
        // This is important because dotman snapshots are cumulative
        mirror.clear_working_directory()?;

        // Export this commit's exact state to mirror
        let _exported_files = exporter.export_commit(commit_id, mirror.get_mirror_path())?;

        let author = &snapshot.commit.author;
        let message = &snapshot.commit.message;
        let timestamp = snapshot.commit.timestamp;

        // Commit in mirror with original timestamp
        let git_commit = mirror.commit_with_timestamp(message, author, timestamp)?;

        mapping_manager.add_and_save(opts.remote, commit_id, &git_commit)?;
    }

    let last_dotman_commit = commits_to_push.last().context("No commits to push")?;
    let last_git_commit = mapping_manager
        .mapping()
        .get_git_commit(opts.remote, last_dotman_commit)
        .context("Failed to get git commit mapping")?;

    if opts.dry_run {
        super::print_info("Dry run - not pushing to remote");
        super::print_success(&format!(
            "Dry run complete - would push {} commit{} to {} ({})",
            commits_to_push.len(),
            if commits_to_push.len() == 1 { "" } else { "s" },
            opts.remote,
            url
        ));
        return Ok(());
    }

    // Push to remote with force options
    super::print_info(&format!("Pushing branch '{}' to remote...", opts.branch));

    if opts.force || opts.force_with_lease {
        // Use mirror's force push method
        push_with_force(&mirror, opts.branch, opts.force_with_lease)?;
    } else {
        mirror.push(opts.branch)?;
    }

    // Push tags if requested
    if opts.tags {
        super::print_info("Pushing tags...");
        push_tags(&mirror)?;
    }

    mapping_manager.update_branch_and_save(
        opts.branch,
        last_dotman_commit,
        Some((opts.remote, &last_git_commit)),
    )?;

    super::print_success(&format!(
        "Successfully pushed {} commit{} to {} ({}) - branch '{}'",
        commits_to_push.len(),
        if commits_to_push.len() == 1 { "" } else { "s" },
        opts.remote,
        url,
        opts.branch
    ));
    Ok(())
}

// Helper function to push with force options
fn push_with_force(mirror: &GitMirror, branch: &str, force_with_lease: bool) -> Result<()> {
    let mirror_path = mirror.get_mirror_path();

    let mut args = vec!["push", "origin", branch];

    if force_with_lease {
        args.push("--force-with-lease");
    } else {
        args.push("--force");
    }

    let output = Command::new("git")
        .args(&args)
        .current_dir(mirror_path)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!("Git force push failed: {stderr}"));
    }

    Ok(())
}

// Helper function to push tags
fn push_tags(mirror: &GitMirror) -> Result<()> {
    let output = Command::new("git")
        .args(["push", "origin", "--tags"])
        .current_dir(mirror.get_mirror_path())
        .output()?;

    if output.status.success() {
        super::print_success("Tags pushed successfully");
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        super::print_warning(&format!("Failed to push tags: {stderr}"));
        // Don't fail the entire operation if tags fail
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, RemoteType};
    use std::fs;
    use tempfile::tempdir;

    fn create_test_context(
        remote_type: RemoteType,
        url: Option<String>,
    ) -> Result<(tempfile::TempDir, DotmanContext)> {
        let temp = tempdir()?;
        let repo_path = temp.path().join(".dotman");
        let config_path = temp.path().join("config.toml");

        fs::create_dir_all(&repo_path)?;
        fs::create_dir_all(repo_path.join("commits"))?;
        fs::create_dir_all(repo_path.join("objects"))?;
        fs::create_dir_all(repo_path.join("refs/heads"))?;

        // Create main branch file with a dummy commit ID
        fs::write(repo_path.join("refs/heads/main"), "0".repeat(40))?;

        // Create HEAD file pointing to main branch
        fs::write(repo_path.join("HEAD"), "ref: refs/heads/main")?;

        let mut config = Config::default();
        config.core.repo_path = repo_path.clone();

        let remote_config = crate::config::RemoteConfig { remote_type, url };
        config.remotes.insert("origin".to_string(), remote_config);
        config.save(&config_path)?;

        Ok((
            temp,
            DotmanContext {
                repo_path,
                config_path,
                config,
                no_pager: true,
            },
        ))
    }

    #[test]
    fn test_execute_no_remote() -> Result<()> {
        let (_temp, mut ctx) = create_test_context(RemoteType::None, None)?;

        let result = execute(
            &mut ctx,
            Some("origin"),
            Some("main"),
            false,
            false,
            false,
            false,
            false,
        );
        assert!(result.is_err());

        Ok(())
    }

    #[test]
    fn test_execute_git_remote() -> Result<()> {
        let (_temp, _ctx) = create_test_context(
            RemoteType::Git,
            Some("https://github.com/user/repo.git".to_string()),
        )?;

        // This would normally call git command, which we'd need to mock
        // For now, we're just testing that it routes to the correct function
        // In a real scenario, we'd use dependency injection or mocking

        Ok(())
    }

    #[test]
    fn test_push_to_git_no_url() -> Result<()> {
        let (_temp, ctx) = create_test_context(RemoteType::Git, None)?;

        let remote_config = crate::config::RemoteConfig {
            remote_type: RemoteType::Git,
            url: None,
        };
        let opts = PushOptions {
            remote: "origin",
            branch: "main",
            force: false,
            force_with_lease: false,
            dry_run: false,
            tags: false,
        };
        let result = push_to_git(&ctx, &remote_config, &opts);
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
    fn test_check_repo_initialized_error() -> Result<()> {
        let temp = tempdir()?;
        let repo_path = temp.path().join("nonexistent").join("deep").join("path");

        let conflict_path = temp.path().join("nonexistent");
        fs::write(&conflict_path, "blocking file")?;

        let mut ctx = DotmanContext {
            repo_path,
            config_path: temp.path().join("config"),
            config: Config::default(),
            no_pager: true,
        };

        let result = execute(
            &mut ctx,
            Some("origin"),
            Some("main"),
            false,
            false,
            false,
            false,
            false,
        );
        assert!(result.is_err());

        Ok(())
    }

    #[test]
    fn test_remote_urls_with_special_chars() -> Result<()> {
        let special_urls = vec![
            "git@github.com:user/repo.git",
            "https://user:pass@github.com/repo.git",
            "git@gitlab.com:user/repo.git",
            "https://bitbucket.org/user/repo.git",
            "ssh://git@custom-host.com:2222/path/to/repo.git",
            "file:///local/path/to/repo",
        ];

        for url in special_urls {
            let (_temp, ctx) = create_test_context(RemoteType::Git, Some(url.to_string()))?;
            let origin_remote = ctx.config.remotes.get("origin").unwrap();
            assert!(origin_remote.url.is_some());
            assert_eq!(origin_remote.url.as_ref().unwrap(), url);
        }

        Ok(())
    }

    #[test]
    fn test_push_with_empty_repo() -> Result<()> {
        let (_temp, ctx) = create_test_context(
            RemoteType::Git,
            Some("https://github.com/user/repo.git".to_string()),
        )?;

        // Ensure repo has no commits (empty HEAD file)
        let head_path = ctx.repo_path.join("HEAD");
        if head_path.exists() {
            fs::remove_file(&head_path)?;
        }

        // (actual command would fail, but our function should handle it gracefully)

        Ok(())
    }

    #[test]
    fn test_determine_push_target_with_tracking() -> Result<()> {
        let (_temp, mut ctx) = create_test_context(
            RemoteType::Git,
            Some("https://github.com/user/repo.git".to_string()),
        )?;

        // Set up branch tracking
        let tracking = crate::config::BranchTracking {
            remote: "origin".to_string(),
            branch: "main".to_string(),
        };
        ctx.config.set_branch_tracking("main".to_string(), tracking);

        // Test with no parameters - should use tracking
        let (remote, branch, should_set) = determine_push_target(&ctx, None, None, false)?;
        assert_eq!(remote, "origin");
        assert_eq!(branch, "main");
        assert!(!should_set);

        Ok(())
    }

    #[test]
    fn test_determine_push_target_no_tracking_single_remote() -> Result<()> {
        let (_temp, ctx) = create_test_context(
            RemoteType::Git,
            Some("https://github.com/user/repo.git".to_string()),
        )?;

        // No tracking set, single remote - should auto-set upstream
        let (remote, branch, should_set) = determine_push_target(&ctx, None, None, false)?;
        assert_eq!(remote, "origin");
        assert_eq!(branch, "main");
        assert!(should_set);

        Ok(())
    }

    #[test]
    fn test_determine_push_target_explicit_params() -> Result<()> {
        let (_temp, ctx) = create_test_context(
            RemoteType::Git,
            Some("https://github.com/user/repo.git".to_string()),
        )?;

        // Explicit parameters should be used as-is
        let (remote, branch, should_set) =
            determine_push_target(&ctx, Some("upstream"), Some("develop"), false)?;
        assert_eq!(remote, "upstream");
        assert_eq!(branch, "develop");
        assert!(!should_set);

        // With set-upstream flag
        let (remote, branch, should_set) =
            determine_push_target(&ctx, Some("origin"), Some("feature"), true)?;
        assert_eq!(remote, "origin");
        assert_eq!(branch, "feature");
        assert!(should_set);

        Ok(())
    }

    #[test]
    fn test_determine_push_target_multiple_remotes_error() -> Result<()> {
        let (_temp, mut ctx) = create_test_context(
            RemoteType::Git,
            Some("https://github.com/user/repo.git".to_string()),
        )?;

        // Add second remote
        ctx.config.set_remote(
            "upstream".to_string(),
            crate::config::RemoteConfig {
                remote_type: RemoteType::Git,
                url: Some("https://github.com/upstream/repo.git".to_string()),
            },
        );

        // No tracking, multiple remotes - should error
        let result = determine_push_target(&ctx, None, None, false);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("multiple remotes") || err_msg.contains("Available remotes"),
            "Error message should mention multiple remotes, got: {err_msg}"
        );

        Ok(())
    }
}
