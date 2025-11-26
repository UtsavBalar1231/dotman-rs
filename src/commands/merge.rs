use crate::DotmanContext;
use crate::dag;
use crate::mapping::MappingManager;
use crate::mirror::GitMirror;
use crate::output;
use crate::refs::{RefManager, resolver::RefResolver};
use crate::storage::index::Index;
use crate::storage::snapshots::SnapshotManager;
use crate::storage::{Commit, FileEntry, file_ops::hash_bytes};
use crate::sync::Importer;
use crate::utils::{
    commit::generate_commit_id, get_current_timestamp, get_precise_timestamp, get_user_from_config,
};
use anyhow::{Context, Result};
use colored::Colorize;
use std::collections::HashMap;
use std::fmt::Write as FmtWrite;
use std::process::{Command, Stdio};

/// Execute merge command - join two or more development histories together
///
/// # Errors
///
/// Returns an error if:
/// - The repository is not initialized
/// - The target branch or commit cannot be resolved
/// - There are conflicts during the merge
/// - The merge operation fails
/// - Fast-forward is not possible when `no_ff` is false
pub fn execute(
    ctx: &DotmanContext,
    branch: &str,
    no_ff: bool,
    squash: bool,
    message: Option<&str>,
) -> Result<()> {
    ctx.check_repo_initialized()?;

    // Check for --continue or --abort flags in branch argument
    if branch == "--continue" {
        return execute_merge_continue(ctx, message);
    }
    if branch == "--abort" {
        return execute_merge_abort(ctx);
    }

    // Resolve the target branch/commit
    let resolver = RefResolver::new(ctx.repo_path.clone());
    let target_commit = if branch.contains('/') {
        // Handle remote branch references like origin/main
        handle_remote_branch_merge(ctx, branch, no_ff, squash, message)?
    } else {
        // Handle local branch/commit
        resolver
            .resolve(branch)
            .with_context(|| format!("Failed to resolve reference: {branch}"))?
    };

    let ref_manager = RefManager::new(ctx.repo_path.clone());
    let current_commit = ref_manager
        .get_head_commit()?
        .context("No commits in current branch")?;

    if current_commit == target_commit {
        output::info("Already up to date.");
        return Ok(());
    }

    let snapshot_manager =
        SnapshotManager::new(ctx.repo_path.clone(), ctx.config.core.compression_level);
    let can_fast_forward = dag::is_ancestor(&snapshot_manager, &current_commit, &target_commit);

    if can_fast_forward && !no_ff && !squash {
        // Fast-forward merge
        output::info(&format!(
            "Fast-forwarding to {}",
            target_commit[..8.min(target_commit.len())].yellow()
        ));

        // Update HEAD to target commit
        if let Some(current_branch) = ref_manager.current_branch()? {
            ref_manager.update_branch(&current_branch, &target_commit)?;
        } else {
            ref_manager.set_head_to_commit(
                &target_commit,
                Some("merge"),
                Some(&format!("merge: fast-forward to {}", &target_commit[..8])),
            )?;
        }

        // Update working directory
        crate::commands::checkout::execute(ctx, &target_commit, false)?;

        output::success(&format!(
            "Fast-forwarded to {}",
            target_commit[..8.min(target_commit.len())].yellow()
        ));
    } else {
        // Three-way merge or squash merge
        if squash {
            perform_squash_merge(ctx, &current_commit, &target_commit, branch, message)?;
        } else {
            perform_three_way_merge(ctx, &current_commit, &target_commit, branch, message)?;
        }
    }

    Ok(())
}

/// Handles merging from remote tracking branches
///
/// This function processes merge requests that reference remote branches (e.g., origin/main).
/// It fetches the latest changes from the remote, imports them if needed, and returns the
/// corresponding commit ID in the local repository.
///
/// # Arguments
///
/// * `ctx` - The dotman context containing repository configuration
/// * `branch_ref` - Remote branch reference in format "remote/branch" (e.g., "origin/main")
/// * `_no_ff` - Whether to force a merge commit (currently unused)
/// * `_squash` - Whether to perform a squash merge (currently unused)
/// * `_message` - Optional custom merge message (currently unused)
///
/// # Returns
///
/// Returns the commit ID of the remote branch in the local repository. If the remote
/// commit has been imported before, returns the existing mapping. Otherwise, imports
/// the remote state and creates a new commit.
///
/// # Errors
///
/// Returns an error if:
/// - The remote branch reference format is invalid
/// - The remote is not configured
/// - The remote has no URL configured
/// - Checkout of the remote branch fails
/// - Import of remote changes fails
fn handle_remote_branch_merge(
    ctx: &DotmanContext,
    branch_ref: &str,
    _no_ff: bool,
    _squash: bool,
    _message: Option<&str>,
) -> Result<String> {
    // Parse remote/branch format
    let parts: Vec<&str> = branch_ref.split('/').collect();
    if parts.len() != 2 {
        return Err(anyhow::anyhow!(
            "Invalid remote branch reference: {branch_ref}"
        ));
    }

    let remote = parts[0];
    let branch = parts[1];

    // Get remote configuration
    let remote_config = ctx
        .config
        .get_remote(remote)
        .with_context(|| format!("Remote '{remote}' does not exist"))?;

    let url = remote_config
        .url
        .as_ref()
        .with_context(|| format!("Remote '{remote}' has no URL configured"))?;

    // Use GitMirror to get the latest commit from remote branch
    let mirror = GitMirror::new(&ctx.repo_path, remote, url, ctx.config.clone());
    mirror.init_mirror()?;

    // Checkout the remote branch in mirror
    let output = Command::new("git")
        .args(["checkout", &format!("origin/{branch}")])
        .current_dir(mirror.get_mirror_path())
        .stdin(Stdio::null())
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!(
            "Failed to checkout remote branch: {stderr}"
        ));
    }

    let git_commit = mirror.get_head_commit()?;

    let mapping_manager = MappingManager::new(&ctx.repo_path)?;
    if let Some(dotman_commit) = mapping_manager
        .mapping()
        .get_dotman_commit(remote, &git_commit)
    {
        return Ok(dotman_commit);
    }

    // Import the remote branch state
    output::info(&format!("Importing {branch_ref} from remote"));

    let mut index = Index::load(&ctx.repo_path.join(crate::INDEX_FILE))?;
    let mut snapshot_manager =
        SnapshotManager::new(ctx.repo_path.clone(), ctx.config.core.compression_level);
    let mut importer = Importer::new(&mut snapshot_manager, &mut index);

    let home_dir = dirs::home_dir().context("Could not find home directory")?;
    let _changes = importer.import_changes(
        mirror.get_mirror_path(),
        &home_dir,
        ctx.config.tracking.follow_symlinks,
    )?;

    let (timestamp, nanos) = get_precise_timestamp();
    let author = get_user_from_config(&ctx.config);
    let message = format!("Import from {branch_ref}");

    // Create tree hash
    let mut tree_content = String::new();
    for (path, entry) in &index.staged_entries {
        #[allow(clippy::expect_used)] // Writing to String never fails
        writeln!(&mut tree_content, "{} {}", entry.hash, path.display())
            .expect("String write should never fail");
    }
    let tree_hash = hash_bytes(tree_content.as_bytes());

    // Generate commit ID
    let commit_id = generate_commit_id(&tree_hash, &[], &message, &author, timestamp, nanos);

    // Create commit object
    let commit = Commit {
        id: commit_id.clone(),
        parents: vec![],
        message,
        author,
        timestamp,
        tree_hash,
    };

    // Create snapshot
    let files: Vec<FileEntry> = index.staged_entries.values().cloned().collect();
    snapshot_manager.create_snapshot(commit, &files, None::<fn(usize)>)?;

    // Update mapping
    let mut mapping_manager = MappingManager::new(&ctx.repo_path)?;
    mapping_manager.add_and_save(remote, &commit_id, &git_commit)?;

    Ok(commit_id)
}

/// Performs a three-way merge between two commits
///
/// This function merges changes from a target branch into the current branch by comparing
/// the files in both commits. When files differ between branches, it detects conflicts
/// and resolves them automatically by taking the incoming version.
///
/// # Arguments
///
/// * `ctx` - The dotman context containing repository configuration
/// * `current_commit` - The commit ID of the current branch (base)
/// * `target_commit` - The commit ID of the branch to merge in (incoming)
/// * `branch` - The name of the branch being merged (for display purposes)
/// * `message` - Optional custom merge commit message
///
/// # Returns
///
/// Returns `Ok(())` on successful merge, updating the index, creating a merge commit,
/// and updating the working directory.
///
/// # Errors
///
/// Returns an error if:
/// - Loading snapshots fails
/// - Creating the merge commit fails
/// - Saving the index fails
/// - Updating the working directory fails
///
/// # Note
///
/// This is a simplified implementation that performs a two-way merge between current
/// and target commits. A proper implementation would find the merge base (common ancestor)
/// and perform a true three-way diff to better detect conflicts and auto-resolve changes.
#[allow(clippy::too_many_lines)] // Complex merge logic requires detailed handling
fn perform_three_way_merge(
    ctx: &DotmanContext,
    current_commit: &str,
    target_commit: &str,
    branch: &str,
    message: Option<&str>,
) -> Result<()> {
    output::info(&format!("Merging {} into current branch", branch.yellow()));

    let snapshot_manager =
        SnapshotManager::new(ctx.repo_path.clone(), ctx.config.core.compression_level);

    // Load both snapshots
    let current_snapshot = snapshot_manager.load_snapshot(current_commit)?;
    let target_snapshot = snapshot_manager.load_snapshot(target_commit)?;

    // Note: This is a simplified three-way merge that doesn't find the common ancestor.
    // A proper implementation would:
    // 1. Find the merge base (common ancestor) using find_common_ancestor()
    // 2. Load the base snapshot
    // 3. Perform a true three-way diff between base, current, and target
    // 4. Apply non-conflicting changes automatically
    // Currently this just does a two-way merge between current and target.

    // Perform three-way merge on files
    let mut merged_files = HashMap::new();
    let mut conflicts = Vec::new();

    // Get all unique file paths
    let mut all_paths = std::collections::HashSet::new();
    all_paths.extend(current_snapshot.files.keys().cloned());
    all_paths.extend(target_snapshot.files.keys().cloned());

    let all_paths_vec: Vec<_> = all_paths.into_iter().collect();
    let mut progress = output::start_progress("Merging files", all_paths_vec.len());

    for (i, path) in all_paths_vec.iter().enumerate() {
        let in_current = current_snapshot.files.contains_key(path);
        let in_target = target_snapshot.files.contains_key(path);

        match (in_current, in_target) {
            (true, true) => {
                // File exists in both - check if they differ
                let current_file = &current_snapshot.files[path];
                let target_file = &target_snapshot.files[path];

                if current_file.hash == target_file.hash {
                    // Same content in both branches
                    merged_files.insert(path.clone(), current_file.clone());
                } else {
                    // Files differ - this is a conflict (simplified)
                    conflicts.push(path.clone());
                    // For now, take the target version (in real implementation, would create conflict markers)
                    merged_files.insert(path.clone(), target_file.clone());
                }
            }
            (true, false) => {
                // File only in current branch
                merged_files.insert(path.clone(), current_snapshot.files[path].clone());
            }
            (false, true) => {
                // File only in target branch
                merged_files.insert(path.clone(), target_snapshot.files[path].clone());
            }
            (false, false) => unreachable!(),
        }
        progress.update(i + 1);
    }
    progress.finish();

    if !conflicts.is_empty() {
        output::warning(&format!(
            "Merge completed with {} conflicts:",
            conflicts.len()
        ));
        for path in &conflicts {
            println!("  {} {}", "conflict:".red(), path.display());
        }
        output::info("Conflicts were auto-resolved by taking the incoming version");
    }

    // Create merge commit
    let (timestamp, nanos) = get_precise_timestamp();
    let author = get_user_from_config(&ctx.config);
    let merge_message = message.map_or_else(|| format!("Merge branch '{branch}'"), String::from);

    // Create tree hash from merged files
    let mut tree_content = String::new();
    for (path, file) in &merged_files {
        #[allow(clippy::expect_used)] // Writing to String never fails
        writeln!(&mut tree_content, "{} {}", file.hash, path.display())
            .expect("String write should never fail");
    }
    let tree_hash = hash_bytes(tree_content.as_bytes());

    // Generate commit ID with BOTH parents (merge commit)
    let parents = vec![current_commit.to_string(), target_commit.to_string()];
    let parent_refs: Vec<&str> = parents.iter().map(String::as_str).collect();
    let commit_id = generate_commit_id(
        &tree_hash,
        &parent_refs,
        &merge_message,
        &author,
        timestamp,
        nanos,
    );

    let commit = Commit {
        id: commit_id.clone(),
        parents,
        message: merge_message,
        author,
        timestamp,
        tree_hash,
    };

    // Convert HashMap to files vector
    let files: Vec<FileEntry> = merged_files
        .into_iter()
        .map(|(path, file)| FileEntry {
            path,
            hash: file.hash,
            size: 0, // Will be updated
            modified: timestamp,
            mode: file.mode,
            cached_hash: None,
        })
        .collect();

    // Save snapshot
    snapshot_manager.create_snapshot(commit, &files, None::<fn(usize)>)?;

    // Clear staging area after creating commit
    let index = Index::new();
    index.save(&ctx.repo_path.join(crate::INDEX_FILE))?;

    // Update HEAD
    let ref_manager = RefManager::new(ctx.repo_path.clone());
    if let Some(current_branch) = ref_manager.current_branch()? {
        ref_manager.update_branch(&current_branch, &commit_id)?;
    } else {
        ref_manager.set_head_to_commit(
            &commit_id,
            Some("merge"),
            Some(&format!("merge: {branch}")),
        )?;
    }

    // Update working directory
    output::info("Updating working directory...");
    crate::commands::checkout::execute(ctx, &commit_id, false)?;

    output::success(&format!(
        "Successfully merged '{}' into current branch",
        branch.yellow()
    ));

    Ok(())
}

/// Performs a squash merge of a branch into the current branch
///
/// This function takes all the changes from the target branch and stages them in the
/// index without creating a commit. This allows the user to review and commit the
/// squashed changes as a single commit, effectively combining all commits from the
/// source branch into one.
///
/// # Arguments
///
/// * `ctx` - The dotman context containing repository configuration
/// * `_current_commit` - The commit ID of the current branch (unused)
/// * `target_commit` - The commit ID of the branch to squash merge
/// * `branch` - The name of the branch being merged (for display purposes)
/// * `message` - Optional message suggesting what commit message to use
///
/// # Returns
///
/// Returns `Ok(())` after staging all changes from the target branch. The user
/// must then run `dot commit` to complete the merge.
///
/// # Errors
///
/// Returns an error if:
/// - Loading the target snapshot fails
/// - Loading the index fails
/// - Saving the updated index fails
fn perform_squash_merge(
    ctx: &DotmanContext,
    _current_commit: &str,
    target_commit: &str,
    branch: &str,
    message: Option<&str>,
) -> Result<()> {
    output::info(&format!(
        "Squash merging {} into current branch",
        branch.yellow()
    ));

    let snapshot_manager =
        SnapshotManager::new(ctx.repo_path.clone(), ctx.config.core.compression_level);

    // Load target snapshot
    let target_snapshot = snapshot_manager.load_snapshot(target_commit)?;

    // Apply changes to index but don't commit
    let mut index = Index::load(&ctx.repo_path.join(crate::INDEX_FILE))?;

    for (path, file) in &target_snapshot.files {
        index.stage_entry(FileEntry {
            path: path.clone(),
            hash: file.hash.clone(),
            size: 0,
            modified: get_current_timestamp(),
            mode: file.mode,
            cached_hash: None,
        });
    }

    index.save(&ctx.repo_path.join(crate::INDEX_FILE))?;

    output::success("Squash merge complete. Changes staged for commit.");
    output::info(&format!(
        "Use 'dot commit -m \"{}\"' to complete the merge",
        message.unwrap_or(&format!("Squashed commit from {branch}"))
    ));

    Ok(())
}

/// Continue a merge after resolving conflicts
///
/// Completes a merge that was stopped due to conflicts by creating a merge commit
/// with the resolved changes currently staged in the index.
///
/// # Arguments
///
/// * `ctx` - The dotman context
/// * `message` - Optional custom commit message (uses saved `MERGE_MSG` if None)
///
/// # Errors
///
/// Returns an error if:
/// - No merge is in progress
/// - Conflict markers are still present in staged files
/// - Creating the merge commit fails
pub fn execute_merge_continue(ctx: &DotmanContext, message: Option<&str>) -> Result<()> {
    use crate::conflicts::{ConflictMarker, MergeState};

    let merge_state = MergeState::new(ctx.repo_path.clone());

    // Check if merge is in progress
    let (_merge_head, saved_message) = merge_state
        .load()?
        .context("No merge in progress. Nothing to continue.")?;

    // Load the index to check for staged changes
    let index = Index::load(&ctx.repo_path.join(crate::INDEX_FILE))?;

    if index.staged_entries.is_empty() {
        return Err(anyhow::anyhow!(
            "No changes staged for merge. Please resolve conflicts and stage files with 'dot add'."
        ));
    }

    // Verify no conflict markers remain in staged files
    let home_dir = dirs::home_dir().context("Could not find home directory")?;
    for path in index.staged_entries.keys() {
        let file_path = home_dir.join(path);
        if file_path.exists() {
            let content = std::fs::read_to_string(&file_path)
                .with_context(|| format!("Failed to read staged file: {}", file_path.display()))?;

            if ConflictMarker::has_markers(&content) {
                return Err(anyhow::anyhow!(
                    "Conflict markers still present in {}\n\
                    Please resolve all conflicts before continuing the merge.",
                    path.display()
                ));
            }
        }
    }

    // Create merge commit
    let (timestamp, nanos) = get_precise_timestamp();
    let author = get_user_from_config(&ctx.config);
    let commit_message = message.map_or(saved_message, String::from);

    let ref_manager = RefManager::new(ctx.repo_path.clone());
    let current_commit = ref_manager
        .get_head_commit()?
        .context("No current commit")?;

    // Create tree hash from staged files
    let mut tree_content = String::new();
    for (path, entry) in &index.staged_entries {
        #[allow(clippy::expect_used)]
        writeln!(&mut tree_content, "{} {}", entry.hash, path.display())
            .expect("String write should never fail");
    }
    let tree_hash = hash_bytes(tree_content.as_bytes());

    // Generate commit ID
    let parents: Vec<String> = vec![current_commit];
    let parent_refs: Vec<&str> = parents.iter().map(String::as_str).collect();
    let commit_id = generate_commit_id(
        &tree_hash,
        &parent_refs,
        &commit_message,
        &author,
        timestamp,
        nanos,
    );

    // Create commit object
    let commit = Commit {
        id: commit_id.clone(),
        parents,
        message: commit_message,
        author,
        timestamp,
        tree_hash,
    };

    // Create snapshot with staged files
    let snapshot_manager =
        SnapshotManager::new(ctx.repo_path.clone(), ctx.config.core.compression_level);

    let files: Vec<FileEntry> = index.staged_entries.values().cloned().collect();
    snapshot_manager.create_snapshot(commit, &files, None::<fn(usize)>)?;

    // Update index - commit staged changes
    let mut index = index;
    index.commit_staged();
    index.save(&ctx.repo_path.join(crate::INDEX_FILE))?;

    // Update HEAD
    if let Some(current_branch) = ref_manager.current_branch()? {
        ref_manager.update_branch(&current_branch, &commit_id)?;
    } else {
        ref_manager.set_head_to_commit(
            &commit_id,
            Some("merge"),
            Some(&format!("merge: continue {}", &commit_id[..8])),
        )?;
    }

    // Clear merge state
    merge_state.clear()?;

    output::success(&format!(
        "Merge completed successfully: {}",
        commit_id[..8].yellow()
    ));

    Ok(())
}

/// Abort an in-progress merge
///
/// Cancels a merge operation that was stopped due to conflicts, restoring
/// the repository state to before the merge began.
///
/// # Arguments
///
/// * `ctx` - The dotman context
///
/// # Errors
///
/// Returns an error if:
/// - No merge is in progress
/// - Failed to restore previous state
pub fn execute_merge_abort(ctx: &DotmanContext) -> Result<()> {
    use crate::conflicts::MergeState;

    let merge_state = MergeState::new(ctx.repo_path.clone());

    // Check if merge is in progress
    if !merge_state.is_merge_in_progress() {
        return Err(anyhow::anyhow!("No merge in progress. Nothing to abort."));
    }

    // Clear merge state files
    merge_state.clear()?;

    // Restore working directory to HEAD
    output::info("Restoring working directory to HEAD...");
    let ref_manager = RefManager::new(ctx.repo_path.clone());
    if let Some(head_commit) = ref_manager.get_head_commit()? {
        crate::commands::checkout::execute(ctx, &head_commit, false)?;
    }

    // Clear any staged changes from the merge
    let mut index = Index::load(&ctx.repo_path.join(crate::INDEX_FILE))?;
    index.staged_entries.clear();
    index.save(&ctx.repo_path.join(crate::INDEX_FILE))?;

    output::success("Merge aborted. Repository restored to pre-merge state.");

    Ok(())
}
