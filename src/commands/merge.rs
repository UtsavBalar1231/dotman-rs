use crate::DotmanContext;
use crate::mapping::MappingManager;
use crate::mirror::GitMirror;
use crate::refs::{RefManager, resolver::RefResolver};
use crate::storage::index::Index;
use crate::storage::snapshots::SnapshotManager;
use crate::storage::{Commit, FileEntry};
use crate::sync::Importer;
use crate::utils::{
    commit::generate_commit_id, get_current_timestamp, get_current_user_with_config,
    hash::hash_bytes,
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
            ref_manager.set_head_to_commit_with_reflog(
                &target_commit,
                "merge",
                &format!("merge: fast-forward to {}", &target_commit[..8]),
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
    let _changes = importer.import_changes(mirror.get_mirror_path(), &home_dir)?;

    let timestamp = get_current_timestamp();
    let author = get_current_user_with_config(&ctx.config);
    let message = format!("Import from {branch_ref}");

    // Create tree hash
    let mut tree_content = String::new();
    for (path, entry) in &index.entries {
        writeln!(&mut tree_content, "{} {}", entry.hash, path.display())
            .expect("String write should never fail");
    }
    let tree_hash = hash_bytes(tree_content.as_bytes());

    // Generate commit ID
    let commit_id = generate_commit_id(&tree_hash, None, &message, &author, timestamp);

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

fn is_ancestor(ctx: &DotmanContext, ancestor: &str, descendant: &str) -> bool {
    let snapshot_manager =
        SnapshotManager::new(ctx.repo_path.clone(), ctx.config.core.compression_level);

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

    // Find common ancestor (simplified - just use parent chains)
    let _common_ancestor = find_common_ancestor(ctx, current_commit, target_commit);

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
    let timestamp = get_current_timestamp();
    let author = get_current_user_with_config(&ctx.config);
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
        ref_manager.set_head_to_commit_with_reflog(
            &commit_id,
            "merge",
            &format!("merge: {branch}"),
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

fn find_common_ancestor(ctx: &DotmanContext, commit1: &str, commit2: &str) -> Option<String> {
    let snapshot_manager =
        SnapshotManager::new(ctx.repo_path.clone(), ctx.config.core.compression_level);

    // Build ancestor sets for both commits
    let ancestors1 = get_all_ancestors(&snapshot_manager, commit1);
    let ancestors2 = get_all_ancestors(&snapshot_manager, commit2);

    // Find common ancestors
    let common: Vec<_> = ancestors1.intersection(&ancestors2).cloned().collect();

    if common.is_empty() {
        return None;
    }

    // Return the first common ancestor (simplified - should find the most recent)
    Some(common[0].clone())
}

fn get_all_ancestors(snapshot_manager: &SnapshotManager, commit: &str) -> HashSet<String> {
    let mut ancestors = HashSet::new();
    let mut to_visit = vec![commit.to_string()];

    while let Some(current) = to_visit.pop() {
        if !ancestors.insert(current.clone()) {
            continue; // Already visited
        }

        if let Ok(snapshot) = snapshot_manager.load_snapshot(&current)
            && let Some(parent) = snapshot.commit.parent
        {
            to_visit.push(parent);
        }
    }

    ancestors
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use std::fs;
    use tempfile::tempdir;

    fn create_test_context() -> Result<DotmanContext> {
        let temp = tempdir()?;
        let repo_path = temp.path().join(".dotman");
        let config_path = temp.path().join("config.toml");

        fs::create_dir_all(&repo_path)?;
        fs::create_dir_all(repo_path.join("commits"))?;
        fs::create_dir_all(repo_path.join("objects"))?;
        fs::create_dir_all(repo_path.join("refs/heads"))?;
        fs::write(repo_path.join("HEAD"), "ref: refs/heads/main")?;

        let config = Config::default();
        config.save(&config_path)?;

        Ok(DotmanContext {
            repo_path,
            config_path,
            config,
            no_pager: true,
        })
    }

    #[test]
    fn test_execute_no_commits() -> Result<()> {
        let ctx = create_test_context()?;

        let result = execute(&ctx, "branch", false, false, None);
        assert!(result.is_err());

        Ok(())
    }

    #[test]
    fn test_is_ancestor_same_commit() -> Result<()> {
        let ctx = create_test_context()?;

        let result = is_ancestor(&ctx, "abc123", "abc123");
        assert!(result); // A commit is its own ancestor

        Ok(())
    }
}
