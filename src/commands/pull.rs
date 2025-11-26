use crate::DotmanContext;
use crate::NULL_COMMIT_ID;
use crate::dag;
use crate::mapping::MappingManager;
use crate::mirror::GitMirror;
use crate::output;
use crate::refs::RefManager;
use crate::storage::index::Index;
use crate::storage::snapshots::SnapshotManager;
use crate::sync::Importer;
use crate::transaction::Transaction;
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

/// Performs the actual git pull operation (fetch + merge/rebase)
///
/// Handles fetching from remote git repository, importing changes to local dotman
/// repository, and performing merge or rebase based on flags. Imports each git commit
/// individually to preserve full commit history with original metadata.
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
    use crate::utils::commit::generate_commit_id;

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

    let remote_head = mirror.get_head_commit()?;

    // Check if we already have this commit mapped (scope ensures lock is released)
    let already_synced = {
        let mapping_manager = MappingManager::new(&ctx.repo_path)?;
        mapping_manager
            .mapping()
            .get_dotman_commit(remote, &remote_head)
    };

    if let Some(dotman_commit) = already_synced {
        // We already have this commit, just checkout
        output::info(&format!(
            "Commit already synchronized, checking out {}",
            &dotman_commit[..8]
        ));

        crate::commands::checkout::execute(ctx, &dotman_commit, false)?;

        output::success(&format!(
            "Successfully pulled from {remote} ({branch}) - already up to date"
        ));
        return Ok(());
    }

    // Find the last git commit we have synced (to determine range of new commits)
    let last_known_git_commit = {
        let mapping_manager = MappingManager::new(&ctx.repo_path)?;
        // Walk back from remote HEAD to find a commit we know
        let all_remote_commits = mirror.list_commits_between(None, &remote_head)?;
        let mut last_known = None;
        for git_id in all_remote_commits.iter().rev() {
            if mapping_manager
                .mapping()
                .get_dotman_commit(remote, git_id)
                .is_some()
            {
                last_known = Some(git_id.clone());
                break;
            }
        }
        last_known
    };

    // Get list of new git commits (oldest first)
    let new_git_commits =
        mirror.list_commits_between(last_known_git_commit.as_deref(), &remote_head)?;

    if new_git_commits.is_empty() {
        output::info("No new commits to import");
        output::success(&format!(
            "Successfully pulled from {remote} ({branch}) - already up to date"
        ));
        return Ok(());
    }

    output::info(&format!(
        "Found {} new commit{} to import",
        new_git_commits.len(),
        if new_git_commits.len() == 1 { "" } else { "s" }
    ));

    // Begin transaction - auto-rollback on drop if not committed
    let mut txn = Transaction::begin(ctx)?;

    let index_path = ctx.repo_path.join(crate::INDEX_FILE);
    let ref_manager = RefManager::new(ctx.repo_path.clone());
    let current_branch = ref_manager.current_branch()?;
    let original_ref = current_branch
        .as_ref()
        .and_then(|b| ref_manager.get_branch_commit(b).ok());

    let home_dir = dirs::home_dir().context("Could not find home directory")?;
    let mut mapping_manager = MappingManager::new(&ctx.repo_path)?;
    let mut last_dotman_commit: Option<String> = None;
    let mut last_git_commit: Option<String> = None;
    let mut total_changes = ImportChangeSummary::default();

    // Import each git commit individually, preserving history
    let mut progress = output::start_progress("Importing commits", new_git_commits.len());
    for (i, git_commit_id) in new_git_commits.iter().enumerate() {
        // Checkout this specific git commit in mirror
        mirror.checkout_commit(git_commit_id)?;

        // Get original commit metadata from git
        let git_info = mirror.get_commit_info(git_commit_id)?;

        // Stage files from mirror at this commit state WITHOUT copying to HOME
        // Files are stored in objects/ and staged in index, but not written to disk
        // The working directory update happens during final checkout
        let mut index = Index::load(&index_path)?;
        let mut snapshot_manager =
            SnapshotManager::new(ctx.repo_path.clone(), ctx.config.core.compression_level);
        let mut importer = Importer::new(&mut snapshot_manager, &mut index);

        let changes = importer.stage_from_directory(
            mirror.get_mirror_path(),
            &home_dir,
            ctx.config.tracking.follow_symlinks,
        )?;

        total_changes.added += changes.added.len();
        total_changes.modified += changes.modified.len();
        total_changes.deleted += changes.deleted.len();

        index.save(&index_path)?;

        // Use original commit message and author from git
        let message = git_info.message.clone();
        let author = format!("{} <{}>", git_info.author_name, git_info.author_email);
        let timestamp = git_info.timestamp;
        let nanos = 0u32; // Git doesn't store nanoseconds

        // Get git parents and map them to dotman commit IDs
        // This preserves the actual git parent structure (including merge commits)
        let git_parents = mirror.get_commit_parents(git_commit_id)?;
        let dotman_parents: Vec<String> = git_parents
            .iter()
            .filter_map(|git_parent| {
                // Look up the dotman commit ID for this git parent
                mapping_manager
                    .mapping()
                    .get_dotman_commit(remote, git_parent)
            })
            .collect();

        // For root commits or if parents aren't imported yet, fall back to local HEAD
        let parents: Vec<String> = if dotman_parents.is_empty() {
            ref_manager
                .get_head_commit()
                .ok()
                .flatten()
                .into_iter()
                .collect()
        } else {
            dotman_parents
        };

        // Create tree hash from all file hashes
        let mut tree_content = String::new();
        for (path, entry) in &index.staged_entries {
            writeln!(tree_content, "{} {}", entry.hash, path.display())?;
        }
        let tree_hash = hash_bytes(tree_content.as_bytes());

        // Generate content-addressed commit ID with ALL parents
        let parent_refs: Vec<&str> = parents.iter().map(String::as_str).collect();
        let commit_id = generate_commit_id(
            &tree_hash,
            &parent_refs,
            &message,
            &author,
            timestamp,
            nanos,
        );

        // Create commit object with proper parent structure
        let commit = Commit {
            id: commit_id.clone(),
            parents,
            message,
            author,
            timestamp,
            tree_hash,
        };

        let files: Vec<FileEntry> = index.staged_entries.values().cloned().collect();

        snapshot_manager.create_snapshot(commit, &files, None::<fn(usize)>)?;

        txn.track_commit(&commit_id);

        index.commit_staged();
        index.save(&index_path)?;

        mapping_manager
            .mapping_mut()
            .add_mapping(remote, &commit_id, git_commit_id);

        txn.track_mapping(remote, &commit_id, git_commit_id);

        last_dotman_commit = Some(commit_id);
        last_git_commit = Some(git_commit_id.clone());

        progress.update(i + 1);
    }
    progress.finish();

    // Save all mappings atomically
    mapping_manager.save()?;

    // Get final commit ID
    let final_commit_id = last_dotman_commit.context("No commits were created")?;
    let final_git_commit = last_git_commit.context("No git commits processed")?;

    // Track remote ref update for rollback (capture old value first)
    let old_remote_ref = ref_manager.get_remote_ref(remote, branch).ok();
    txn.track_remote_ref(remote, branch, old_remote_ref.as_deref());

    // Update remote tracking ref (NOT local branch - that happens after merge)
    ref_manager.update_remote_ref(remote, branch, &final_commit_id)?;

    // Update branch mapping
    mapping_manager.update_branch_and_save(
        branch,
        &final_commit_id,
        Some((remote, &final_git_commit)),
    )?;

    // Check fast-forward status using DAG ancestry
    let snapshot_manager =
        SnapshotManager::new(ctx.repo_path.clone(), ctx.config.core.compression_level);
    let is_fast_forward = match &original_ref {
        None => true,
        Some(local_head) if local_head == NULL_COMMIT_ID => true,
        Some(local_head) => dag::is_ancestor(&snapshot_manager, local_head, &final_commit_id),
    };

    // Drop mapping_manager to release the file lock before merge operations
    drop(mapping_manager);

    output::success(&format!(
        "Successfully pulled {} commit{} from {} ({}) - {}",
        new_git_commits.len(),
        if new_git_commits.len() == 1 { "" } else { "s" },
        remote,
        branch,
        total_changes.summary()
    ));

    // Detect conflicts before merging
    let conflicts_detected = detect_merge_conflicts(ctx, &final_commit_id, branch)?;

    if !conflicts_detected {
        // No conflicts - proceed with merge strategies
        if rebase {
            output::info("Rebasing local changes on top of pulled changes...");
            perform_rebase(ctx, &final_commit_id)?;
        } else if no_ff || squash {
            output::info(&format!(
                "Merging with {} strategy...",
                if squash { "squash" } else { "no-ff" }
            ));
            crate::commands::merge::execute(
                ctx,
                &format!("{remote}/{branch}"),
                no_ff,
                squash,
                None,
            )?;
        } else if is_fast_forward {
            output::info("Fast-forwarding...");
            // Update local branch to point to pulled commit
            if let Some(branch_name) = &current_branch {
                ref_manager.update_branch(branch_name, &final_commit_id)?;
            }
            // Checkout to restore files from the pulled commit
            let target = current_branch.as_deref().unwrap_or(&final_commit_id);
            crate::commands::checkout::execute(ctx, target, true)?;
        } else {
            output::info("Merging divergent histories...");
            crate::commands::merge::execute(
                ctx,
                &format!("{remote}/{branch}"),
                false,
                false,
                None,
            )?;
        }
    }

    // All operations succeeded - commit transaction to prevent rollback
    txn.commit()?;
    Ok(())
}

/// Summary of import changes across multiple commits
#[derive(Default)]
struct ImportChangeSummary {
    /// Number of files added
    added: usize,
    /// Number of files modified
    modified: usize,
    /// Number of files deleted
    deleted: usize,
}

impl ImportChangeSummary {
    /// Generate a human-readable summary of the changes
    fn summary(&self) -> String {
        let mut parts = Vec::new();
        if self.added > 0 {
            parts.push(format!("{} added", self.added));
        }
        if self.modified > 0 {
            parts.push(format!("{} modified", self.modified));
        }
        if self.deleted > 0 {
            parts.push(format!("{} deleted", self.deleted));
        }
        if parts.is_empty() {
            "No file changes".to_string()
        } else {
            parts.join(", ")
        }
    }
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

    // If there's no current commit or it's the null commit, there can't be conflicts
    let Some(current_commit_id) = current_commit else {
        return Ok(false);
    };
    if current_commit_id == NULL_COMMIT_ID {
        return Ok(false);
    }

    let current_snapshot = snapshot_manager.load_snapshot(&current_commit_id)?;
    let target_snapshot = snapshot_manager.load_snapshot(target_commit)?;

    // Try to find common ancestor for proper three-way merge
    let common_ancestor =
        dag::find_common_ancestor(&snapshot_manager, &current_commit_id, target_commit)
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

/// Performs rebase operation onto a specific commit
///
/// This delegates to the rebase module to replay local commits on top of the
/// new base commit (`onto_commit`).
///
/// # Arguments
///
/// * `ctx` - The dotman context
/// * `onto_commit` - The commit ID to rebase onto
///
/// # Errors
///
/// Returns an error if the rebase operation fails
fn perform_rebase(ctx: &DotmanContext, onto_commit: &str) -> Result<()> {
    crate::commands::rebase::execute_start_internal(ctx, onto_commit)
}
