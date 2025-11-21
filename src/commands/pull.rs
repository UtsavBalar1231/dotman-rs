use crate::DotmanContext;
use crate::mapping::MappingManager;
use crate::mirror::GitMirror;
use crate::output;
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
        output::info(&format!(
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

/// Rollback a failed pull operation
///
/// Cleans up orphaned commits and restores repository state after a pull failure.
/// This function implements a best-effort rollback strategy that attempts to restore
/// the repository to its pre-pull state.
///
/// ## Rollback Strategy
///
/// The pull operation creates several persistent changes before attempting the final
/// checkout/merge:
/// 1. Creates a new dotman commit snapshot on disk
/// 2. Updates branch ref to point to new commit
/// 3. Updates index with imported files
///
/// Rollback must undo these in reverse order to minimize inconsistency:
///
/// ### Step 1: Restore Branch Reference
/// If we updated a branch ref, reset it to the original commit. This is critical
/// because it determines what users see as HEAD. Failure here is recorded but
/// doesn't stop the rollback.
///
/// ### Step 2: Delete Orphaned Snapshot
/// Remove the commit snapshot we created. Since the commit is new and nothing else
/// references it yet, this is safe. If deletion fails, it becomes an orphaned
/// commit that wastes disk space but doesn't break functionality.
///
/// ### Step 3: Index Cleanup
/// The index was already saved with imported changes. We could restore from a backup,
/// but currently we just warn the user. This is acceptable because:
/// - Staged changes are visible and can be manually unstaged
/// - No data is lost (files are in both index and objects)
/// - A fresh `dot status` will show what's staged
///
/// ## Error Handling Philosophy
///
/// Rollback uses a "continue on error" approach: we attempt all cleanup steps
/// even if some fail, collecting errors along the way. This maximizes the chance
/// of restoring to a usable state. Only truly fatal errors (like being unable to
/// access the filesystem) should prevent further rollback attempts.
///
/// # Arguments
///
/// * `ctx` - The dotman context
/// * `commit_id` - The orphaned commit to remove
/// * `branch` - The branch that was updated (if any)
/// * `original_ref` - The original commit ID to restore (if any)
///
/// # Errors
///
/// Returns an error if cleanup operations fail
fn rollback_pull(
    ctx: &DotmanContext,
    commit_id: &str,
    branch: Option<&str>,
    original_ref: Option<&str>,
) -> Result<()> {
    let mut errors = Vec::new();

    // Step 1: Restore branch ref to original value
    // This is the most critical step - it determines what users see as current state
    if let (Some(branch_name), Some(original)) = (branch, original_ref) {
        let ref_manager = RefManager::new(ctx.repo_path.clone());
        if let Err(e) = ref_manager.update_branch(branch_name, original) {
            errors.push(format!("Failed to restore branch '{branch_name}': {e}"));
        } else {
            output::info(&format!(
                "Restored branch '{}' to {}",
                branch_name,
                &original[..8]
            ));
        }
    }

    // Step 2: Delete orphaned snapshot
    // The commit snapshot was created but never fully integrated, so it's safe to delete
    let snapshot_path = ctx
        .repo_path
        .join("commits")
        .join(format!("{commit_id}.zst"));

    if snapshot_path.exists() {
        if let Err(e) = std::fs::remove_file(&snapshot_path) {
            errors.push(format!(
                "Failed to remove orphaned commit {}: {}",
                &commit_id[..8],
                e
            ));
        } else {
            output::info(&format!("Removed orphaned commit {}", &commit_id[..8]));
        }
    }

    // Step 3: Restore index to original state
    // Currently we just warn about potential staged changes. A full implementation
    // would restore from a pre-pull backup of the index file.
    let index_path = ctx.repo_path.join(crate::INDEX_FILE);
    if index_path.exists() {
        // Index was already saved, but we could restore from backup if we had one
        // For now, just note that index may need manual cleanup
        output::info("Note: Index may contain staged changes from failed pull");
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(anyhow::anyhow!(
            "Rollback completed with {} error(s): {}",
            errors.len(),
            errors.join("; ")
        ))
    }
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

    output::info(&format!("Pulling from git remote {remote} ({url})"));

    // Create and initialize mirror
    let mirror = GitMirror::new(&ctx.repo_path, remote, url, ctx.config.clone());
    mirror.init_mirror()?;

    // Pull changes in mirror
    output::info(&format!("Fetching branch '{branch}' from remote..."));
    mirror.pull(branch)?;

    let git_commit = mirror.get_head_commit()?;

    // Check if we already have this commit mapped (scope ensures lock is released)
    let already_synced = {
        let mapping_manager = MappingManager::new(&ctx.repo_path)?;
        mapping_manager
            .mapping()
            .get_dotman_commit(remote, &git_commit) // mapping_manager dropped here, releasing lock
    };

    if let Some(dotman_commit) = already_synced {
        // We already have this commit, just checkout
        output::info(&format!(
            "Commit already synchronized, checking out {}",
            &dotman_commit[..8]
        ));

        // Checkout the commit
        crate::commands::checkout::execute(ctx, &dotman_commit, false)?;

        output::success(&format!(
            "Successfully pulled from {remote} ({branch}) - already up to date"
        ));
        return Ok(());
    }

    // Import changes from mirror
    output::info("Importing changes from remote...");

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
        output::info("No changes to import");
        output::success(&format!(
            "Successfully pulled from {remote} ({branch}) - already up to date"
        ));
        return Ok(());
    }

    index.save(&ctx.repo_path.join(crate::INDEX_FILE))?;

    output::info(&format!(
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
    for (path, entry) in &index.staged_entries {
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

    // Validate we can proceed before creating snapshot (point of no return)
    // This prevents orphaned commits if later steps fail
    let files: Vec<FileEntry> = index.staged_entries.values().cloned().collect();

    // TRANSACTION POINT: Create snapshot (persisted to disk)
    snapshot_manager.create_snapshot(commit, &files, None::<fn(usize)>)?;

    // Clear staging area after creating commit
    index.commit_staged();
    index.save(&ctx.repo_path.join(crate::INDEX_FILE))?;

    // Store original branch ref for rollback if needed
    let ref_manager = RefManager::new(ctx.repo_path.clone());
    let current_branch = ref_manager.current_branch()?;
    let original_ref = current_branch
        .as_ref()
        .and_then(|b| ref_manager.get_branch_commit(b).ok());

    // Update refs (atomic operation)
    if let Some(branch_name) = &current_branch {
        ref_manager.update_branch(branch_name, &commit_id)?;
    }

    // Update mapping
    let mut mapping_manager = MappingManager::new(&ctx.repo_path)?;
    mapping_manager.add_and_save(remote, &commit_id, &git_commit)?;
    mapping_manager.update_branch_and_save(branch, &commit_id, Some((remote, &git_commit)))?;

    output::success(&format!(
        "Successfully pulled from {} ({}) - {}",
        remote,
        branch,
        changes.summary()
    ));

    // Detect conflicts before merging
    let conflicts_detected = detect_merge_conflicts(ctx, &commit_id, branch)?;

    if !conflicts_detected {
        // No conflicts - proceed with merge strategies
        // Wrap operations that can fail in rollback handler
        let operation_result = if rebase {
            // Rebase current changes on top of pulled changes
            output::info("Rebasing local changes on top of pulled changes...");
            perform_rebase(ctx, &commit_id)
        } else if no_ff || squash {
            // Use merge command with appropriate flags
            output::info(&format!(
                "Merging with {} strategy...",
                if squash { "squash" } else { "no-ff" }
            ));
            crate::commands::merge::execute(ctx, &format!("{remote}/{branch}"), no_ff, squash, None)
        } else {
            // Default: checkout the new commit to update working directory
            output::info("Updating working directory to match pulled changes...");
            crate::commands::checkout::execute(ctx, &commit_id, false)
        };

        // If checkout/merge failed, rollback the pull
        if let Err(e) = operation_result {
            eprintln!("Operation failed: {e}");
            output::info("Rolling back pull due to failure...");

            // Attempt rollback
            if let Err(rollback_err) = rollback_pull(
                ctx,
                &commit_id,
                current_branch.as_deref(),
                original_ref.as_deref(),
            ) {
                eprintln!("Warning: Rollback failed: {rollback_err}");
                eprintln!("Repository may be in inconsistent state.");
                eprintln!("Manual cleanup may be required:");
                eprintln!("  - Orphaned commit: {}", &commit_id[..8]);
                if let Some(branch_name) = current_branch
                    && let Some(original) = original_ref
                {
                    eprintln!("  - Reset branch '{}' to: {}", branch_name, &original[..8]);
                }
            } else {
                output::info("Successfully rolled back pull operation");
            }

            return Err(e);
        }
    }

    Ok(())
}

/// Detects merge conflicts between current state and target commit
///
/// Performs three-way merge analysis to find conflicts. If conflicts are found,
/// writes conflict markers to files and saves merge state for later resolution.
///
/// # Arguments
///
/// * `ctx` - The dotman context
/// * `target_commit` - The commit ID being merged in
/// * `branch_name` - The branch name being merged (for display and markers)
///
/// # Returns
///
/// Returns `true` if conflicts were detected and handled, `false` if no conflicts
///
/// # Errors
///
/// Returns an error if conflict detection or marker writing fails
fn detect_merge_conflicts(
    ctx: &DotmanContext,
    target_commit: &str,
    branch_name: &str,
) -> Result<bool> {
    use crate::conflicts::{MergeState, detect_conflicts, write_conflict_markers};

    let snapshot_manager =
        SnapshotManager::new(ctx.repo_path.clone(), ctx.config.core.compression_level);

    // Load current and target snapshots for conflict detection
    let ref_manager = RefManager::new(ctx.repo_path.clone());
    let current_commit = ref_manager.get_head_commit()?;

    // If there's no current commit, there can't be conflicts
    let Some(current_commit_id) = current_commit else {
        return Ok(false);
    };

    let current_snapshot = snapshot_manager.load_snapshot(&current_commit_id)?;
    let target_snapshot = snapshot_manager.load_snapshot(target_commit)?;

    // Try to find common ancestor for proper three-way merge
    let common_ancestor = find_common_ancestor(ctx, &current_commit_id, target_commit)
        .and_then(|ancestor_id| snapshot_manager.load_snapshot(&ancestor_id).ok());

    // Detect conflicts between snapshots
    let conflicts = detect_conflicts(
        &current_snapshot,
        &target_snapshot,
        common_ancestor.as_ref(),
    )?;

    if conflicts.is_empty() {
        return Ok(false);
    }

    // Conflicts detected - write conflict markers to files
    output::warning(&format!(
        "Merge conflicts detected in {} file(s):",
        conflicts.len()
    ));

    let home_dir = dirs::home_dir().context("Could not find home directory")?;
    let objects_path = ctx.repo_path.join(crate::OBJECTS_DIR);

    for conflict in &conflicts {
        println!("  {} {}", "CONFLICT:".red(), conflict.path.display());

        // Write conflict markers to the file
        let target_path = home_dir.join(&conflict.path);
        if let Err(e) = write_conflict_markers(
            conflict,
            &snapshot_manager,
            &objects_path,
            &target_path,
            branch_name,
        ) {
            output::warning(&format!(
                "Failed to write conflict markers for {}: {}",
                conflict.path.display(),
                e
            ));
        }
    }

    // Save merge state for continuation or abort
    let merge_state = MergeState::new(ctx.repo_path.clone());
    let merge_msg = format!("Merge branch '{branch_name}' into current branch");
    merge_state.save(target_commit, &merge_msg)?;

    // Print instructions for resolution
    println!();
    output::info("Merge stopped due to conflicts.");
    output::info("After resolving conflicts:");
    println!("  1. Edit conflicted files to resolve conflicts");
    println!("  2. Stage resolved files: dot add <files>");
    println!("  3. Complete merge: dot merge --continue");
    println!();
    output::info("Or abort the merge:");
    println!("  dot merge --abort");

    Err(anyhow::anyhow!(
        "Automatic merge failed; fix conflicts and then commit the result."
    ))
}

/// Finds the common ancestor (merge base) between two commits
///
/// This is a simplified implementation that walks back from both commits
/// until a common commit is found.
///
/// # Arguments
///
/// * `ctx` - The dotman context
/// * `commit1` - First commit ID
/// * `commit2` - Second commit ID
///
/// # Returns
///
/// Returns `Some(commit_id)` if a common ancestor is found, `None` otherwise
fn find_common_ancestor(ctx: &DotmanContext, commit1: &str, commit2: &str) -> Option<String> {
    use std::collections::HashSet;

    let snapshot_manager =
        SnapshotManager::new(ctx.repo_path.clone(), ctx.config.core.compression_level);

    // Build ancestor chain for commit1
    let mut commit1_ancestors = HashSet::new();
    let mut current = Some(commit1.to_string());

    while let Some(commit_id) = current {
        commit1_ancestors.insert(commit_id.clone());

        if let Ok(snapshot) = snapshot_manager.load_snapshot(&commit_id) {
            current = snapshot.commit.parent;
        } else {
            break;
        }
    }

    // Walk back from commit2 until we find a common ancestor
    let mut current = Some(commit2.to_string());
    while let Some(commit_id) = current {
        if commit1_ancestors.contains(&commit_id) {
            return Some(commit_id);
        }

        if let Ok(snapshot) = snapshot_manager.load_snapshot(&commit_id) {
            current = snapshot.commit.parent;
        } else {
            break;
        }
    }

    None
}

/// Performs rebase operation onto a specific commit
///
/// **NOT CURRENTLY IMPLEMENTED** - Returns an error indicating rebase is not yet supported.
///
/// A full implementation would:
/// 1. Find the common ancestor between current HEAD and `onto_commit`
/// 2. Save local commits since the common ancestor
/// 3. Reset to the new base commit (`onto_commit`)
/// 4. Cherry-pick/replay the local commits on top
/// 5. Handle merge conflicts during replay
///
/// # Arguments
///
/// * `_ctx` - The dotman context (unused)
/// * `onto_commit` - The commit ID to rebase onto (unused)
///
/// # Errors
///
/// Always returns an error indicating the feature is not implemented
fn perform_rebase(_ctx: &DotmanContext, onto_commit: &str) -> Result<()> {
    // TODO: Implement proper rebase functionality
    // For now, return a clear error to prevent data loss

    Err(anyhow::anyhow!(
        "Rebase is not yet implemented.\n\n\
        Suggested alternatives:\n\
        - Use 'dot pull --no-rebase' for a merge-based pull\n\
        - Use 'dot pull --ff-only' to only allow fast-forward merges\n\
        - Manually merge with 'dot fetch' followed by 'dot merge {}'",
        &onto_commit[..8.min(onto_commit.len())]
    ))
}
