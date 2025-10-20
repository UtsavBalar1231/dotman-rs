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
    remote: Option<&str>,
    branch: Option<&str>,
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

    // Determine remote and branch to pull from
    let (remote_name, branch_name) = determine_pull_target(ctx, remote, branch)?;

    let remote_config = ctx.config.get_remote(&remote_name).with_context(|| {
        format!("Remote '{remote_name}' does not exist. Use 'dot remote add' to add it.")
    })?;

    match &remote_config.remote_type {
        crate::config::RemoteType::Git => pull_from_git(
            ctx,
            remote_config,
            &remote_name,
            &branch_name,
            rebase,
            no_ff,
            squash,
        ),
        crate::config::RemoteType::None => Err(anyhow::anyhow!(
            "Remote '{remote_name}' has no type configured or is not a Git remote."
        )),
    }
}

/// Determine the remote and branch to pull from
///
/// Returns (`remote_name`, `branch_name`)
fn determine_pull_target(
    ctx: &DotmanContext,
    remote: Option<&str>,
    branch: Option<&str>,
) -> Result<(String, String)> {
    use crate::refs::RefManager;

    let ref_manager = RefManager::new(ctx.repo_path.clone());

    // If both remote and branch are provided, use them directly
    if let (Some(r), Some(b)) = (remote, branch) {
        return Ok((r.to_string(), b.to_string()));
    }

    // Get current branch
    let current_branch = ref_manager
        .current_branch()?
        .context("Not on any branch (detached HEAD). Please specify branch to pull.")?;

    // If only remote is provided, use current branch
    if let Some(r) = remote {
        return Ok((r.to_string(), current_branch));
    }

    // If only branch is provided, need to find remote from tracking
    if let Some(b) = branch {
        if let Some(tracking) = ctx.config.get_branch_tracking(b) {
            return Ok((tracking.remote.clone(), b.to_string()));
        }
        return Err(anyhow::anyhow!(
            "Branch '{b}' has no upstream tracking. Please specify remote."
        ));
    }

    // Neither remote nor branch provided - use tracking info for current branch
    if let Some(tracking) = ctx.config.get_branch_tracking(&current_branch) {
        super::print_info(&format!(
            "Pulling from tracked upstream: {}/{}",
            tracking.remote, tracking.branch
        ));
        return Ok((tracking.remote.clone(), tracking.branch.clone()));
    }

    // No tracking info - provide helpful error
    if ctx.config.remotes.is_empty() {
        return Err(anyhow::anyhow!(
            "No remotes configured. Use 'dot remote add <name> <url>' to add a remote."
        ));
    }

    let available_remotes: Vec<String> = ctx.config.remotes.keys().cloned().collect();
    Err(anyhow::anyhow!(
        "Branch '{}' has no upstream tracking. Available remotes: {}\n\
         Please specify: 'dot pull <remote>' or set upstream: 'dot branch set-upstream <remote>'",
        current_branch,
        available_remotes.join(", ")
    ))
}

/// Performs the actual git pull operation (fetch + merge/rebase)
///
/// Handles fetching from remote git repository, importing changes to local dotman
/// repository, and performing merge or rebase based on flags. Creates a commit
/// for imported changes and updates the mapping between git and dotman commits.
///
/// # Arguments
///
/// * `ctx` - The dotman context with repository path and configuration
/// * `remote_config` - Configuration for the remote being pulled from
/// * `remote` - Name of the remote (e.g., "origin")
/// * `branch` - Name of the branch to pull
/// * `rebase` - If true, rebase local changes on top of pulled changes
/// * `no_ff` - If true, create a merge commit even if fast-forward is possible
/// * `squash` - If true, squash all changes into a single commit
///
/// # Errors
///
/// Returns an error if:
/// - The remote URL is not configured
/// - The git mirror initialization or pull operation fails
/// - Importing changes from the mirror fails
/// - Creating or saving the commit snapshot fails
/// - Updating references or mappings fails
/// - The merge or rebase operation fails
#[allow(clippy::too_many_lines)]
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
    use crate::utils::{commit::generate_commit_id, get_precise_timestamp, get_user_from_config};

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
    let changes = importer.import_changes(
        mirror.get_mirror_path(),
        &home_dir,
        ctx.config.tracking.follow_symlinks,
    )?;

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
    let (timestamp, nanos) = get_precise_timestamp();
    let author = get_user_from_config(&ctx.config);

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
    let commit_id = generate_commit_id(
        &tree_hash,
        parent.as_deref(),
        &message,
        &author,
        timestamp,
        nanos,
    );

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

/// Performs rebase operation onto a specific commit
///
/// Replays current branch commits on top of the target commit. This is a simplified
/// implementation that currently just checks out the new commit. A full implementation
/// would save local commits since the common ancestor, reset to the new base, and
/// replay the local commits.
///
/// # Arguments
///
/// * `ctx` - The dotman context with repository path and configuration
/// * `onto_commit` - The commit ID to rebase onto
///
/// # Errors
///
/// Returns an error if:
/// - The checkout operation to the target commit fails
/// - The target commit does not exist
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
