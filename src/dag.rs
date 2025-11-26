//! DAG (Directed Acyclic Graph) utilities for commit traversal.
//!
//! This module provides functions for traversing the commit DAG, including
//! ancestry detection and common ancestor finding for proper merge operations.

use crate::NULL_COMMIT_ID;
use crate::storage::snapshots::SnapshotManager;
use std::collections::{HashSet, VecDeque};

/// Determines if a fast-forward merge is possible between two commits.
///
/// Used by merge/pull/push to check if the target commit contains all history
/// from the source commit. If `ancestor` is in `descendant`'s history, we can
/// fast-forward (just move the ref) instead of creating a merge commit.
///
/// Uses BFS through ALL parents to handle merge commits correctly - a commit
/// is reachable if it appears anywhere in the DAG, not just the first-parent chain.
///
/// # Arguments
///
/// * `snapshot_manager` - Snapshot manager to load commit history
/// * `ancestor` - The potential ancestor commit ID
/// * `descendant` - The commit that might have `ancestor` in its history
///
/// # Returns
///
/// `true` if `ancestor` is reachable from `descendant`, `false` otherwise.
#[must_use]
pub fn is_ancestor(snapshot_manager: &SnapshotManager, ancestor: &str, descendant: &str) -> bool {
    if ancestor == descendant {
        return true;
    }

    // NULL_COMMIT_ID is never an ancestor of anything meaningful
    if ancestor == NULL_COMMIT_ID {
        return false;
    }

    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();
    queue.push_back(descendant.to_string());

    while let Some(commit_id) = queue.pop_front() {
        if commit_id == ancestor {
            return true;
        }

        // Skip NULL_COMMIT_ID and already visited
        if commit_id == NULL_COMMIT_ID || !visited.insert(commit_id.clone()) {
            continue;
        }

        if let Ok(snapshot) = snapshot_manager.load_snapshot(&commit_id) {
            // Enqueue ALL parents for BFS traversal
            for parent in &snapshot.commit.parents {
                if parent != NULL_COMMIT_ID && !visited.contains(parent) {
                    queue.push_back(parent.clone());
                }
            }
        }
    }

    false
}

/// Builds the complete history set for a commit (used by `find_common_ancestor`).
///
/// Returns all commits reachable from `start` by following parent links.
/// Includes `start` itself. Traverses all parents to handle merge commits.
///
/// # Arguments
///
/// * `snapshot_manager` - Snapshot manager to load commit history
/// * `start` - The commit ID to start traversal from (included in result)
///
/// # Returns
///
/// A set of all commit IDs reachable from `start` via parent links.
#[must_use]
pub fn collect_ancestors(snapshot_manager: &SnapshotManager, start: &str) -> HashSet<String> {
    let mut ancestors = HashSet::new();
    let mut queue = VecDeque::new();
    queue.push_back(start.to_string());

    while let Some(commit_id) = queue.pop_front() {
        // Skip NULL_COMMIT_ID
        if commit_id == NULL_COMMIT_ID {
            continue;
        }

        if !ancestors.insert(commit_id.clone()) {
            continue; // Already visited
        }

        if let Ok(snapshot) = snapshot_manager.load_snapshot(&commit_id) {
            for parent in &snapshot.commit.parents {
                if parent != NULL_COMMIT_ID && !ancestors.contains(parent) {
                    queue.push_back(parent.clone());
                }
            }
        }
    }

    ancestors
}

/// Finds the merge base for three-way merge operations.
///
/// When merging two branches, we need the most recent commit that's in both
/// histories - this is the "base" for computing what changed on each branch.
///
/// # Arguments
///
/// * `snapshot_manager` - Snapshot manager to load commit history
/// * `commit1` - First commit ID to find common ancestor with
/// * `commit2` - Second commit ID to find common ancestor with
///
/// # Returns
///
/// The best common ancestor (most recent, not dominated by another), or `None`
/// if there is no common ancestor.
#[must_use]
pub fn find_common_ancestor(
    snapshot_manager: &SnapshotManager,
    commit1: &str,
    commit2: &str,
) -> Option<String> {
    // Handle edge cases
    if commit1 == commit2 {
        return Some(commit1.to_string());
    }

    if commit1 == NULL_COMMIT_ID || commit2 == NULL_COMMIT_ID {
        return None;
    }

    // Collect all ancestors of both commits
    let ancestors1 = collect_ancestors(snapshot_manager, commit1);
    let ancestors2 = collect_ancestors(snapshot_manager, commit2);

    // Find intersection (common ancestors)
    let common: HashSet<_> = ancestors1.intersection(&ancestors2).cloned().collect();

    if common.is_empty() {
        return None;
    }

    // Find the "best" common ancestor (closest to both commits)
    // This is the one where no other common ancestor is a descendant of it
    find_best_common_ancestor(snapshot_manager, &common)
}

/// Find the best common ancestor from a set of common ancestors.
///
/// The "best" ancestor is one where no other common ancestor is its descendant.
/// This is the most recent common ancestor in the DAG.
fn find_best_common_ancestor(
    snapshot_manager: &SnapshotManager,
    common: &HashSet<String>,
) -> Option<String> {
    // For each candidate, check if any other common ancestor is a descendant of it
    for candidate in common {
        let mut is_best = true;
        for other in common {
            if candidate != other && is_ancestor(snapshot_manager, candidate, other) {
                // `other` is a descendant of `candidate`, so `other` is more recent
                is_best = false;
                break;
            }
        }
        if is_best {
            return Some(candidate.clone());
        }
    }
    // Fallback: return any common ancestor
    common.iter().next().cloned()
}

/// Build commit chain following first parent only (for push/log operations).
///
/// This follows the convention that the first parent is the "mainline" in merge commits.
///
/// # Arguments
/// * `snapshot_manager` - The snapshot manager to load commits
/// * `start` - The commit ID to start from
///
/// # Returns
/// A vector of commit IDs from `start` to the root, following first parents.
#[must_use]
pub fn build_first_parent_chain(snapshot_manager: &SnapshotManager, start: &str) -> Vec<String> {
    let mut chain = Vec::new();
    let mut current = Some(start.to_string());
    let mut visited = HashSet::new();

    while let Some(commit_id) = current {
        if commit_id == NULL_COMMIT_ID || !visited.insert(commit_id.clone()) {
            break;
        }

        chain.push(commit_id.clone());

        current = snapshot_manager
            .load_snapshot(&commit_id)
            .ok()
            .and_then(|s| s.commit.parents.first().cloned())
            .filter(|p| p != NULL_COMMIT_ID);
    }

    chain
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_ancestor_same_commit_returns_true() {
        // is_ancestor(sm, X, X) should return true without loading snapshots
        // This test verifies the early-return optimization
        let temp = tempfile::TempDir::new().unwrap();
        let sm = SnapshotManager::new(temp.path().to_path_buf(), 3);
        assert!(is_ancestor(&sm, "abc123", "abc123"));
    }

    #[test]
    fn test_null_commit_is_never_ancestor() {
        let temp = tempfile::TempDir::new().unwrap();
        let sm = SnapshotManager::new(temp.path().to_path_buf(), 3);
        // NULL_COMMIT_ID should never be an ancestor of anything
        assert!(!is_ancestor(&sm, NULL_COMMIT_ID, "abc123"));
    }

    #[test]
    fn test_find_common_ancestor_same_commit() {
        let temp = tempfile::TempDir::new().unwrap();
        let sm = SnapshotManager::new(temp.path().to_path_buf(), 3);
        // Same commit is its own common ancestor
        assert_eq!(
            find_common_ancestor(&sm, "abc123", "abc123"),
            Some("abc123".to_string())
        );
    }

    #[test]
    fn test_find_common_ancestor_null_commit() {
        let temp = tempfile::TempDir::new().unwrap();
        let sm = SnapshotManager::new(temp.path().to_path_buf(), 3);
        // NULL_COMMIT_ID has no common ancestor with anything
        assert_eq!(find_common_ancestor(&sm, NULL_COMMIT_ID, "abc123"), None);
        assert_eq!(find_common_ancestor(&sm, "abc123", NULL_COMMIT_ID), None);
    }
}
