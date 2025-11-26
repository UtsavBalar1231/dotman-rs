use crate::DotmanContext;
use crate::dag;
use crate::mapping::{CommitMapping, MappingManager};
use crate::mirror::GitMirror;
use crate::output;
use crate::refs::RefManager;
use crate::storage::index::Index;
use crate::storage::snapshots::SnapshotManager;
use crate::sync::Exporter;
use anyhow::{Context, Result};
use std::path::Path;
use std::process::{Command, Stdio};

/// Public arguments for push command
#[allow(clippy::struct_excessive_bools)]
pub struct PushArgs {
    /// Name of the remote repository
    pub remote: Option<String>,
    /// Name of the branch to push
    pub branch: Option<String>,
    /// Force push even if not fast-forward
    pub force: bool,
    /// Safer force push that checks remote state
    pub force_with_lease: bool,
    /// Preview push without actually sending changes
    pub dry_run: bool,
    /// Whether to push tags along with commits
    pub tags: bool,
    /// Set tracking relationship with upstream
    pub set_upstream: bool,
}

/// Options for push operation to remote repository (internal use)
#[allow(clippy::struct_excessive_bools)]
struct PushOptions<'a> {
    /// Name of the remote repository
    remote: &'a str,
    /// Name of the branch to push
    branch: &'a str,
    /// Force push even if not fast-forward
    force: bool,
    /// Safer force push that checks remote state
    force_with_lease: bool,
    /// Preview push without actually sending changes
    dry_run: bool,
    /// Whether to push tags along with commits
    tags: bool,
}

/// Reset git mirror to previous HEAD state
///
/// Used for cleanup when push fails after git commits were created.
fn reset_mirror_head(mirror: &GitMirror, previous_head: Option<&str>) -> Result<()> {
    if let Some(head) = previous_head {
        let output = Command::new("git")
            .args(["reset", "--hard", head])
            .current_dir(mirror.get_mirror_path())
            .stdin(Stdio::null())
            .output()
            .context("Failed to execute git reset")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("Git reset failed: {stderr}"));
        }
    } else {
        mirror.clear_working_directory()?;
    }
    Ok(())
}

/// Verify that remote repository received the pushed commits
///
/// Uses git ls-remote to check that the remote branch contains the expected commit.
/// This provides assurance that the push actually succeeded at the protocol level.
///
/// # Arguments
///
/// * `mirror` - The git mirror to verify
/// * `branch` - The branch name that was pushed
/// * `expected_commit` - The git commit ID we expect to see on the remote
///
/// # Errors
///
/// Returns an error if:
/// - git ls-remote command fails
/// - Remote branch doesn't exist
/// - Remote branch doesn't contain the expected commit
fn verify_remote_push(mirror: &GitMirror, branch: &str, expected_commit: &str) -> Result<()> {
    let output = Command::new("git")
        .args(["ls-remote", "origin", branch])
        .current_dir(mirror.get_mirror_path())
        .output()
        .context("Failed to execute git ls-remote for verification")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!("Git ls-remote failed: {stderr}"));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    // ls-remote output format: <commit-id>\t<ref-name>
    // Example: abc123def456\trefs/heads/main
    let remote_commit = stdout
        .lines()
        .find(|line| line.contains(&format!("refs/heads/{branch}")))
        .and_then(|line| line.split_whitespace().next())
        .context("Remote branch not found in ls-remote output")?;

    // Verify the remote has our commit
    if !remote_commit.starts_with(expected_commit) && !expected_commit.starts_with(remote_commit) {
        return Err(anyhow::anyhow!(
            "Remote branch '{}' has commit {} but expected {}",
            branch,
            &remote_commit[..8.min(remote_commit.len())],
            &expected_commit[..8.min(expected_commit.len())]
        ));
    }

    output::success(&format!(
        "Verified remote has commit {}",
        &expected_commit[..8.min(expected_commit.len())]
    ));

    Ok(())
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
pub fn execute(ctx: &mut DotmanContext, args: &PushArgs) -> Result<()> {
    ctx.check_repo_initialized()?;

    if args.force && args.force_with_lease {
        return Err(anyhow::anyhow!(
            "Cannot use both --force and --force-with-lease"
        ));
    }

    // Determine remote and branch to use
    let (remote_name, branch_name, should_set_upstream) = determine_push_target(
        ctx,
        args.remote.as_deref(),
        args.branch.as_deref(),
        args.set_upstream,
    )?;

    let remote_config = ctx.config.get_remote(&remote_name).with_context(|| {
        format!("Remote '{remote_name}' does not exist. Use 'dot remote add' to add it.")
    })?;

    if args.dry_run {
        output::info(&format!(
            "Dry run mode - would push to {remote_name} ({branch_name})"
        ));
    }

    let push_opts = PushOptions {
        remote: &remote_name,
        branch: &branch_name,
        force: args.force,
        force_with_lease: args.force_with_lease,
        dry_run: args.dry_run,
        tags: args.tags,
    };

    let result = match &remote_config.remote_type {
        crate::config::RemoteType::Git => push_to_git(ctx, remote_config, &push_opts),
        crate::config::RemoteType::None => Err(anyhow::anyhow!(
            "Remote '{remote_name}' has no type configured or is not a Git remote."
        )),
    };

    // Auto-set upstream on successful push if needed
    if result.is_ok() && should_set_upstream && !args.dry_run {
        let tracking = crate::config::BranchTracking {
            remote: remote_name.clone(),
            branch: branch_name.clone(),
        };
        ctx.config
            .set_branch_tracking(branch_name.clone(), tracking);
        ctx.config.save(&ctx.config_path)?;
        output::info(&format!(
            "Branch '{branch_name}' set up to track '{remote_name}/{branch_name}'"
        ));
    }

    result
}

/// Build a chain of commits from root to the given commit
fn build_commit_chain(snapshot_manager: &SnapshotManager, target_commit: &str) -> Vec<String> {
    let mut commits = Vec::new();
    let mut current_commit = Some(target_commit.to_string());

    // Follow parent links to collect all commits (newest to oldest)
    while let Some(ref commit_id) = current_commit {
        // Skip the special "all zeros" parent that represents no parent
        if commit_id.chars().all(|c| c == '0') {
            break;
        }

        commits.push(commit_id.clone());

        match snapshot_manager.load_snapshot(commit_id) {
            Ok(snapshot) => {
                current_commit = snapshot.commit.parents.first().cloned();
            }
            Err(_) => {
                // Stop if we can't load a commit
                break;
            }
        }
    }

    // Reverse to get oldest-first order (root → target)
    // This is more reliable than timestamp sorting when commits have the same timestamp
    commits.reverse();
    commits
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
            "Branch '{b}' has no upstream tracking. Please specify remote or use --set-upstream."
        ));
    }

    // Neither remote nor branch provided - use tracking info for current branch
    if let Some(tracking) = ctx.config.get_branch_tracking(&current_branch) {
        output::info(&format!(
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
        #[allow(clippy::unwrap_used)] // Safe: len() == 1 guarantees next() returns Some
        let (remote_name, _) = ctx.config.remotes.iter().next().unwrap();
        output::info(&format!(
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

/// Performs the actual git push operation
///
/// This function handles the core push workflow:
/// 1. Initializes a git mirror repository
/// 2. Identifies unpushed commits
/// 3. Exports each commit to the mirror
/// 4. Commits changes to git with original timestamps
/// 5. Pushes to the remote repository
///
/// # Arguments
///
/// * `ctx` - The dotman context containing repository configuration
/// * `remote_config` - Configuration for the target remote repository
/// * `opts` - Push options specifying behavior (force, dry-run, etc.)
///
/// # Errors
///
/// Returns an error if:
/// - The remote URL is not configured
/// - Mirror initialization fails
/// - Commit export or mapping fails
/// - Git push operation fails
/// - Tag pushing fails (when requested)
#[allow(clippy::too_many_lines)]
fn push_to_git(
    ctx: &DotmanContext,
    remote_config: &crate::config::RemoteConfig,
    opts: &PushOptions,
) -> Result<()> {
    let url = remote_config
        .url
        .as_ref()
        .with_context(|| format!("Remote '{}' has no URL configured", opts.remote))?;

    output::info(&format!("Pushing to git remote {} ({})", opts.remote, url));

    if opts.force {
        output::warning("Force push requested - this may overwrite remote changes!");
    } else if opts.force_with_lease {
        output::info("Using --force-with-lease for safer force push");
    }

    let mirror = GitMirror::new(&ctx.repo_path, opts.remote, url, ctx.config.clone());
    mirror.init_mirror()?;

    mirror.checkout_branch(opts.branch)?;

    // Fetch latest remote state for divergence detection
    output::info("Fetching remote state...");
    mirror.fetch(Some(opts.branch))?;

    let ref_manager = RefManager::new(ctx.repo_path.clone());
    let current_commit = ref_manager
        .get_head_commit()?
        .context("No commits to push")?;

    let snapshot_manager =
        SnapshotManager::new(ctx.repo_path.clone(), ctx.config.core.compression_level);
    let mut mapping_manager = MappingManager::new(&ctx.repo_path)?;

    // Check for divergent history before proceeding
    // If remote has commits we don't know about, or they're not in our ancestry, reject unless force push
    let remote_git_commit = mirror.get_remote_branch_commit(opts.branch)?;
    if let Some(ref remote_commit) = remote_git_commit {
        // Check if the remote commit is in our mappings
        let remote_dotman_commit = mapping_manager
            .mapping()
            .get_dotman_commit(opts.remote, remote_commit);

        // Check if remote commit is an ancestor of our current commit (fast-forward safe)
        let is_fast_forward = remote_dotman_commit.as_ref().is_some_and(|dotman_id| {
            dag::is_ancestor(&snapshot_manager, dotman_id, &current_commit)
        });

        if !is_fast_forward && !opts.force && !opts.force_with_lease {
            let hint = if remote_dotman_commit.is_some() {
                "Hint: Your local branch has diverged from the remote (rebased or reset)."
            } else {
                "Hint: The remote branch has commits not in your history."
            };
            return Err(anyhow::anyhow!(
                "Updates were rejected because the tip of your current branch is behind its remote counterpart.\n\
                 {hint}\n\
                 Hint: Use 'dot pull' to integrate remote changes, or\n\
                 Hint: Use 'dot push --force' to overwrite remote (may lose data).",
            ));
        }

        if !is_fast_forward && opts.force {
            output::warning(&format!(
                "Remote '{}' has commits not in your ancestry - force push will overwrite them!",
                opts.branch
            ));
        }
    }

    let commits_to_push = get_unpushed_commits(
        &snapshot_manager,
        &mapping_manager,
        opts.remote,
        &current_commit,
    );

    if commits_to_push.is_empty() {
        output::info("Already up to date - no new commits to push");
        return Ok(());
    }

    output::info(&format!(
        "Found {} new commit{} to push",
        commits_to_push.len(),
        if commits_to_push.len() == 1 { "" } else { "s" }
    ));

    // Export and commit each dotman commit to git
    let index = Index::load(&ctx.repo_path.join(crate::INDEX_FILE))?;
    let exporter = Exporter::new(&snapshot_manager, &index);

    // Capture mirror HEAD for rollback if push fails
    let mirror_head_before = mirror.get_head_commit().ok();

    // Collect mappings in memory - only save after push succeeds
    let mut pending_mappings: Vec<(String, String)> = Vec::new();

    let mut progress = output::start_progress("Processing commits", commits_to_push.len());
    for (i, commit_id) in commits_to_push.iter().enumerate() {
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

        // Store mapping in memory only (don't save yet!)
        pending_mappings.push((commit_id.clone(), git_commit));

        progress.update(i + 1);
    }
    progress.finish();

    // Get last git commit from pending mappings (not saved yet)
    let last_git_commit = pending_mappings
        .last()
        .map(|(_, git_commit)| git_commit.clone())
        .context("No commits to push")?;

    if opts.dry_run {
        output::info("Dry run - not pushing to remote");
        output::success(&format!(
            "Dry run complete - would push {} commit{} to {} ({})",
            commits_to_push.len(),
            if commits_to_push.len() == 1 { "" } else { "s" },
            opts.remote,
            url
        ));
        return Ok(());
    }

    // Push to remote
    output::info(&format!("Pushing branch '{}' to remote...", opts.branch));

    let push_result = if opts.force || opts.force_with_lease {
        // Push with force options
        let mirror_path = mirror.get_mirror_path();

        let mut args: Vec<String> = vec![
            "push".to_string(),
            "origin".to_string(),
            opts.branch.to_string(),
        ];

        if opts.force_with_lease {
            // Use --force-with-lease without explicit expected value
            // Git uses the tracking ref from our recent fetch (line 377) automatically
            // This avoids race condition from stale expected value captured earlier
            args.push("--force-with-lease".to_string());
        } else {
            args.push("--force".to_string());
        }

        let output = Command::new("git")
            .args(&args)
            .current_dir(mirror_path)
            .stdin(Stdio::null())
            .output()
            .context("Failed to execute git push")?;

        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(anyhow::anyhow!("Git force push failed: {stderr}"))
        }
    } else {
        mirror.push(opts.branch)
    };

    // Handle push failure - reset mirror to previous state
    if let Err(e) = push_result {
        output::warning("Push failed - resetting mirror...");
        let _ = reset_mirror_head(&mirror, mirror_head_before.as_deref());
        return Err(e);
    }

    // Verify remote received commits
    output::info("Verifying remote received commits...");
    if let Err(e) = verify_remote_push(&mirror, opts.branch, &last_git_commit) {
        output::warning(&format!("Remote verification failed: {e}"));
        let _ = reset_mirror_head(&mirror, mirror_head_before.as_deref());
        return Err(anyhow::anyhow!(
            "Push verification failed - changes rolled back: {e}"
        ));
    }

    for (dotman_commit, git_commit) in pending_mappings {
        mapping_manager
            .mapping_mut()
            .add_mapping(opts.remote, &dotman_commit, &git_commit);
    }

    // Save mappings - if this fails, we still rolled forward (remote has commits)
    // but we warn the user about the inconsistency
    if let Err(e) = mapping_manager.save() {
        output::warning(&format!(
            "Push succeeded but failed to save mappings: {e}\n\
             You may need to re-push to recreate mappings."
        ));
        return Err(e);
    }

    // Get last dotman commit for remote ref and branch mapping
    let last_dotman_commit = commits_to_push.last().context("No commits to push")?;

    // Update remote tracking ref to point to the last dotman commit we pushed
    ref_manager.update_remote_ref(opts.remote, opts.branch, last_dotman_commit)?;

    // Push tags if requested (non-fatal)
    if opts.tags {
        output::info("Pushing tags...");
        if let Err(e) = push_tags(
            &ctx.repo_path,
            &mirror,
            mapping_manager.mapping(),
            opts.remote,
        ) {
            output::warning(&format!("Failed to push tags: {e}"));
            // Don't fail the entire operation if tags fail
        }
    }

    mapping_manager.update_branch_and_save(
        opts.branch,
        last_dotman_commit,
        Some((opts.remote, &last_git_commit)),
    )?;

    output::success(&format!(
        "Successfully pushed {} commit{} to {} ({}) - branch '{}'",
        commits_to_push.len(),
        if commits_to_push.len() == 1 { "" } else { "s" },
        opts.remote,
        url,
        opts.branch
    ));
    Ok(())
}

/// Pushes tags to remote repository
///
/// Executes a git push with --tags flag to push all local tags
/// to the remote. This is a non-fatal operation - if tag pushing
/// fails, a warning is printed but the function still returns Ok.
///
/// # Arguments
///
/// * `repo_path` - Path to the dotman repository
/// * `mirror` - The git mirror instance to use for pushing
/// * `mapping` - The commit mapping to look up git commit IDs
/// * `remote` - The remote name for mapping lookups
///
/// # Errors
///
/// Returns an error if the git command fails to execute (not if the
/// push itself is rejected - that only produces a warning).
fn push_tags(
    repo_path: &Path,
    mirror: &GitMirror,
    mapping: &CommitMapping,
    remote: &str,
) -> Result<()> {
    // Get all dotman tags
    let ref_manager = RefManager::new(repo_path.to_path_buf());
    let tags = ref_manager.list_tags()?;

    if tags.is_empty() {
        output::info("No tags to push");
        return Ok(());
    }

    let mut synced_count = 0;

    // Sync each dotman tag to the git mirror
    for tag_name in &tags {
        // Get the dotman commit this tag points to
        let dotman_commit = match ref_manager.get_tag_commit(tag_name) {
            Ok(commit) => commit,
            Err(e) => {
                output::warning(&format!("Skipping tag '{tag_name}': {e}"));
                continue;
            }
        };

        // Look up the corresponding git commit
        if let Some(git_commit) = mapping.get_git_commit(remote, &dotman_commit) {
            // Create the tag in the mirror
            if let Err(e) = mirror.create_tag(tag_name, &git_commit) {
                output::warning(&format!("Failed to create tag '{tag_name}' in mirror: {e}"));
                continue;
            }
            synced_count += 1;
        } else {
            output::warning(&format!(
                "Tag '{tag_name}' points to commit {} which hasn't been pushed to '{remote}'",
                &dotman_commit[..8]
            ));
        }
    }

    if synced_count == 0 {
        output::info("No tags synced to mirror (commits not pushed yet)");
        return Ok(());
    }

    // Now push tags to remote
    let output = Command::new("git")
        .args(["push", "origin", "--tags"])
        .current_dir(mirror.get_mirror_path())
        .output()?;

    if output.status.success() {
        output::success(&format!(
            "Pushed {} tag{} successfully",
            synced_count,
            if synced_count == 1 { "" } else { "s" }
        ));
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        output::warning(&format!("Failed to push tags: {stderr}"));
        // Don't fail the entire operation if tags fail
    }

    Ok(())
}
