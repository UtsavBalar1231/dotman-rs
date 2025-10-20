use crate::DotmanContext;
use crate::mapping::MappingManager;
use crate::mirror::GitMirror;
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
use std::collections::{HashMap, HashSet};
use std::fmt::Write as FmtWrite;

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
        super::print_info("Already up to date.");
        return Ok(());
    }

    let can_fast_forward = is_ancestor(ctx, &current_commit, &target_commit);

    if can_fast_forward && !no_ff && !squash {
        // Fast-forward merge
        super::print_info(&format!(
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

        super::print_success(&format!(
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
            "Invalid remote branch reference: {}",
            branch_ref
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
    let output = std::process::Command::new("git")
        .args(["checkout", &format!("origin/{branch}")])
        .current_dir(mirror.get_mirror_path())
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!(
            "Failed to checkout remote branch: {}",
            stderr
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
    super::print_info(&format!("Importing {branch_ref} from remote"));

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
    for (path, entry) in &index.entries {
        writeln!(&mut tree_content, "{} {}", entry.hash, path.display())
            .expect("String write should never fail");
    }
    let tree_hash = hash_bytes(tree_content.as_bytes());

    // Generate commit ID
    let commit_id = generate_commit_id(&tree_hash, None, &message, &author, timestamp, nanos);

    // Create commit object
    let commit = Commit {
        id: commit_id.clone(),
        parent: None,
        message,
        author,
        timestamp,
        tree_hash,
    };

    // Create snapshot
    let files: Vec<FileEntry> = index.entries.values().cloned().collect();
    snapshot_manager.create_snapshot(commit, &files)?;

    // Update mapping
    let mut mapping_manager = MappingManager::new(&ctx.repo_path)?;
    mapping_manager.add_and_save(remote, &commit_id, &git_commit)?;

    Ok(commit_id)
}

/// Checks if one commit is an ancestor of another
///
/// This function determines if a commit (ancestor) appears in the history of another
/// commit (descendant) by walking back through the commit chain. It is used to determine
/// whether a fast-forward merge is possible.
///
/// # Arguments
///
/// * `ctx` - The dotman context containing repository configuration
/// * `ancestor` - The potential ancestor commit ID
/// * `descendant` - The descendant commit ID to check
///
/// # Returns
///
/// Returns `true` if `ancestor` is found in the parent chain of `descendant`,
/// `false` otherwise.
///
/// # Note
///
/// This is a simplified implementation that only follows first-parent chains.
/// A complete implementation would need to handle multiple parents (merge commits)
/// and build a full commit graph for accurate ancestry detection.
fn is_ancestor(ctx: &DotmanContext, ancestor: &str, descendant: &str) -> bool {
    let snapshot_manager =
        SnapshotManager::new(ctx.repo_path.clone(), ctx.config.core.compression_level);

    // Note: This is a simplified implementation that only follows first-parent chains.
    // A complete implementation would need to handle multiple parents (merge commits)
    // and build a full commit graph for accurate ancestry detection.

    // Walk back from descendant to see if we reach ancestor
    let mut current = Some(descendant.to_string());
    let mut visited = HashSet::new();

    while let Some(commit_id) = current {
        if commit_id == ancestor {
            return true;
        }

        if !visited.insert(commit_id.clone()) {
            break; // Cycle detected
        }

        // Get parent
        if let Ok(snapshot) = snapshot_manager.load_snapshot(&commit_id) {
            current = snapshot.commit.parent;
        } else {
            break;
        }
    }

    false
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
    super::print_info(&format!("Merging {} into current branch", branch.yellow()));

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

    for path in all_paths {
        let in_current = current_snapshot.files.contains_key(&path);
        let in_target = target_snapshot.files.contains_key(&path);

        match (in_current, in_target) {
            (true, true) => {
                // File exists in both - check if they differ
                let current_file = &current_snapshot.files[&path];
                let target_file = &target_snapshot.files[&path];

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
                merged_files.insert(path.clone(), current_snapshot.files[&path].clone());
            }
            (false, true) => {
                // File only in target branch
                merged_files.insert(path.clone(), target_snapshot.files[&path].clone());
            }
            (false, false) => unreachable!(),
        }
    }

    if !conflicts.is_empty() {
        super::print_warning(&format!(
            "Merge completed with {} conflicts:",
            conflicts.len()
        ));
        for path in &conflicts {
            println!("  {} {}", "conflict:".red(), path.display());
        }
        super::print_info("Conflicts were auto-resolved by taking the incoming version");
    }

    // Create merge commit
    let (timestamp, nanos) = get_precise_timestamp();
    let author = get_user_from_config(&ctx.config);
    let merge_message = message.map_or_else(|| format!("Merge branch '{branch}'"), String::from);

    // Create tree hash from merged files
    let mut tree_content = String::new();
    for (path, file) in &merged_files {
        writeln!(&mut tree_content, "{} {}", file.hash, path.display())
            .expect("String write should never fail");
    }
    let tree_hash = hash_bytes(tree_content.as_bytes());

    // Generate commit ID (with current commit as parent)
    let commit_id = generate_commit_id(
        &tree_hash,
        Some(current_commit),
        &merge_message,
        &author,
        timestamp,
        nanos,
    );

    // Create commit object with both parents (simplified - only tracking one parent in current structure)
    let commit = Commit {
        id: commit_id.clone(),
        parent: Some(current_commit.to_string()),
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
    snapshot_manager.create_snapshot(commit, &files)?;

    // Update index
    let mut index = Index::new();
    for file in &files {
        index.add_entry(file.clone());
    }
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
    super::print_info("Updating working directory...");
    crate::commands::checkout::execute(ctx, &commit_id, false)?;

    super::print_success(&format!(
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
    super::print_info(&format!(
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
        index.add_entry(FileEntry {
            path: path.clone(),
            hash: file.hash.clone(),
            size: 0,
            modified: get_current_timestamp(),
            mode: file.mode,
            cached_hash: None,
        });
    }

    index.save(&ctx.repo_path.join(crate::INDEX_FILE))?;

    super::print_success("Squash merge complete. Changes staged for commit.");
    super::print_info(&format!(
        "Use 'dot commit -m \"{}\"' to complete the merge",
        message.unwrap_or(&format!("Squashed commit from {branch}"))
    ));

    Ok(())
}
