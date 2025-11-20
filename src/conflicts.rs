//! Conflict detection and resolution for merge operations
//!
//! This module provides functionality for detecting conflicts during three-way merges,
//! generating conflict markers in files, and managing merge state persistence.

use crate::storage::index::Index;
use crate::storage::snapshots::{Snapshot, SnapshotManager};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

/// Information about a single merge conflict
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictInfo {
    /// Path to the conflicting file
    pub path: PathBuf,
    /// Content hash from the local/current branch
    pub local_hash: String,
    /// Content hash from the remote/target branch
    pub remote_hash: String,
    /// Content hash from the common ancestor (merge base), if it exists
    pub base_hash: Option<String>,
}

/// Generator for Git-style conflict markers in files
pub struct ConflictMarker;

impl ConflictMarker {
    /// Generate conflict markers for a file with conflicting content
    ///
    /// Creates a string with Git-style conflict markers:
    /// ```text
    /// <<<<<<< HEAD (local)
    /// [local content]
    /// =======
    /// [remote content]
    /// >>>>>>> branch_name (remote)
    /// ```
    ///
    /// # Arguments
    ///
    /// * `local_content` - Content from the current/local branch
    /// * `remote_content` - Content from the incoming/remote branch
    /// * `branch_name` - Name of the branch being merged in
    ///
    /// # Returns
    ///
    /// String containing the conflicted content with markers
    #[must_use]
    pub fn generate(local_content: &str, remote_content: &str, branch_name: &str) -> String {
        format!(
            "<<<<<<< HEAD (local)\n{}\n=======\n{}\n>>>>>>> {} (remote)\n",
            local_content.trim_end(),
            remote_content.trim_end(),
            branch_name
        )
    }

    /// Check if a file contains conflict markers
    ///
    /// # Arguments
    ///
    /// * `content` - File content to check
    ///
    /// # Returns
    ///
    /// `true` if conflict markers are present, `false` otherwise
    #[must_use]
    pub fn has_markers(content: &str) -> bool {
        content.contains("<<<<<<<") && content.contains("=======") && content.contains(">>>>>>>")
    }
}

/// Detects conflicts between current state, remote changes, and common ancestor
///
/// Performs a three-way merge analysis to identify files that have conflicting
/// changes in both the local and remote branches since their common ancestor.
///
/// # Arguments
///
/// * `current_index` - The current working tree index
/// * `remote_snapshot` - Snapshot of the remote/target branch
/// * `common_ancestor` - Snapshot of the merge base (common ancestor), if available
///
/// # Returns
///
/// Vector of `ConflictInfo` structures describing each conflicting file
///
/// # Errors
///
/// Returns an error if file comparisons fail
pub fn detect_conflicts(
    current_index: &Index,
    remote_snapshot: &Snapshot,
    common_ancestor: Option<&Snapshot>,
) -> Result<Vec<ConflictInfo>> {
    let mut conflicts = Vec::new();

    // Get all unique file paths across current, remote, and base
    let mut all_paths = HashSet::new();
    all_paths.extend(current_index.entries.keys().cloned());
    all_paths.extend(remote_snapshot.files.keys().cloned());
    if let Some(base) = common_ancestor {
        all_paths.extend(base.files.keys().cloned());
    }

    for path in all_paths {
        let in_current = current_index.entries.get(&path);
        let in_remote = remote_snapshot.files.get(&path);
        let in_base = common_ancestor.and_then(|base| base.files.get(&path));

        // Three-way merge logic
        let has_conflict = match (in_current, in_remote, in_base) {
            // Both modified from base - check if they differ
            (Some(current), Some(remote), Some(base)) => {
                let current_changed = current.hash != base.hash;
                let remote_changed = remote.hash != base.hash;

                // Conflict if both changed but to different values
                current_changed && remote_changed && current.hash != remote.hash
            }
            // File added in both branches with different content
            (Some(current), Some(remote), None) => current.hash != remote.hash,
            // File deleted in one branch but modified in another
            (Some(current), None, Some(base)) => {
                // Remote deleted, check if we modified
                current.hash != base.hash
            }
            (None, Some(remote), Some(base)) => {
                // Local deleted, check if remote modified
                remote.hash != base.hash
            }
            // No conflict in other cases
            _ => false,
        };

        if has_conflict {
            let local_hash = in_current.map(|e| e.hash.clone()).unwrap_or_default();
            let remote_hash = in_remote.map(|f| f.hash.clone()).unwrap_or_default();
            let base_hash = in_base.map(|f| f.hash.clone());

            conflicts.push(ConflictInfo {
                path: path.clone(),
                local_hash,
                remote_hash,
                base_hash,
            });
        }
    }

    Ok(conflicts)
}

/// Manages merge state persistence for conflict resolution and merge resumption
pub struct MergeState {
    /// Path to the dotman repository
    repo_path: PathBuf,
}

impl MergeState {
    /// Create a new merge state manager
    ///
    /// # Arguments
    ///
    /// * `repo_path` - Path to the dotman repository
    #[must_use]
    pub const fn new(repo_path: PathBuf) -> Self {
        Self { repo_path }
    }

    /// Save merge state to enable continuation or abort
    ///
    /// Creates `MERGE_HEAD` and `MERGE_MSG` files in the repository to track
    /// an ongoing merge operation.
    ///
    /// # Arguments
    ///
    /// * `merge_head` - Commit ID being merged in
    /// * `merge_msg` - Merge commit message
    ///
    /// # Errors
    ///
    /// Returns an error if files cannot be written
    pub fn save(&self, merge_head: &str, merge_msg: &str) -> Result<()> {
        let merge_head_path = self.repo_path.join("MERGE_HEAD");
        let merge_msg_path = self.repo_path.join("MERGE_MSG");

        fs::write(&merge_head_path, merge_head).with_context(|| {
            format!("Failed to write MERGE_HEAD: {}", merge_head_path.display())
        })?;

        fs::write(&merge_msg_path, merge_msg)
            .with_context(|| format!("Failed to write MERGE_MSG: {}", merge_msg_path.display()))?;

        Ok(())
    }

    /// Load merge state from repository
    ///
    /// # Returns
    ///
    /// Returns `Some((merge_head, merge_msg))` if a merge is in progress,
    /// `None` otherwise
    ///
    /// # Errors
    ///
    /// Returns an error if files exist but cannot be read
    pub fn load(&self) -> Result<Option<(String, String)>> {
        let merge_head_path = self.repo_path.join("MERGE_HEAD");
        let merge_msg_path = self.repo_path.join("MERGE_MSG");

        if !merge_head_path.exists() || !merge_msg_path.exists() {
            return Ok(None);
        }

        let merge_head = fs::read_to_string(&merge_head_path)
            .with_context(|| format!("Failed to read MERGE_HEAD: {}", merge_head_path.display()))?;

        let merge_msg = fs::read_to_string(&merge_msg_path)
            .with_context(|| format!("Failed to read MERGE_MSG: {}", merge_msg_path.display()))?;

        Ok(Some((merge_head.trim().to_string(), merge_msg)))
    }

    /// Clear merge state after completion or abort
    ///
    /// # Errors
    ///
    /// Returns an error if files cannot be deleted
    pub fn clear(&self) -> Result<()> {
        let merge_head_path = self.repo_path.join("MERGE_HEAD");
        let merge_msg_path = self.repo_path.join("MERGE_MSG");

        if merge_head_path.exists() {
            fs::remove_file(&merge_head_path).with_context(|| {
                format!("Failed to remove MERGE_HEAD: {}", merge_head_path.display())
            })?;
        }

        if merge_msg_path.exists() {
            fs::remove_file(&merge_msg_path).with_context(|| {
                format!("Failed to remove MERGE_MSG: {}", merge_msg_path.display())
            })?;
        }

        Ok(())
    }

    /// Check if a merge is currently in progress
    #[must_use]
    pub fn is_merge_in_progress(&self) -> bool {
        self.repo_path.join("MERGE_HEAD").exists()
    }
}

/// Write conflict markers to a file in the working tree
///
/// This function retrieves the content from both the local and remote versions
/// of a conflicted file, generates conflict markers, and writes the marked-up
/// content to the working tree.
///
/// # Arguments
///
/// * `conflict` - Information about the conflict
/// * `snapshot_manager` - Manager for loading file content from snapshots
/// * `objects_path` - Path to the objects directory for content retrieval
/// * `target_path` - Path where the conflict-marked file should be written
/// * `branch_name` - Name of the branch being merged (for marker labels)
///
/// # Errors
///
/// Returns an error if:
/// - File content cannot be retrieved from object storage
/// - The conflict-marked file cannot be written
pub fn write_conflict_markers(
    conflict: &ConflictInfo,
    _snapshot_manager: &SnapshotManager,
    objects_path: &Path,
    target_path: &Path,
    branch_name: &str,
) -> Result<()> {
    // Read content from object storage
    let local_content = if conflict.local_hash.is_empty() {
        String::from("(file deleted in local)")
    } else {
        read_object_content(objects_path, &conflict.local_hash)?
    };

    let remote_content = if conflict.remote_hash.is_empty() {
        String::from("(file deleted in remote)")
    } else {
        read_object_content(objects_path, &conflict.remote_hash)?
    };

    // Generate conflict markers
    let marked_content = ConflictMarker::generate(&local_content, &remote_content, branch_name);

    // Write to target path
    if let Some(parent) = target_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
    }

    fs::write(target_path, marked_content).with_context(|| {
        format!(
            "Failed to write conflict markers: {}",
            target_path.display()
        )
    })?;

    Ok(())
}

/// Read content from object storage
///
/// # Arguments
///
/// * `objects_path` - Path to the objects directory
/// * `hash` - Content hash of the object to read
///
/// # Returns
///
/// String content of the object
///
/// # Errors
///
/// Returns an error if the object cannot be read or decoded
fn read_object_content(objects_path: &Path, hash: &str) -> Result<String> {
    // Object storage uses the first 2 characters as directory, rest as filename
    let (dir, file) = if hash.len() >= 2 {
        (&hash[..2], &hash[2..])
    } else {
        return Err(anyhow::anyhow!("Invalid hash: too short"));
    };

    let object_path = objects_path.join(dir).join(file);

    // Read and decompress object content (objects may be compressed)
    let content = fs::read(&object_path)
        .with_context(|| format!("Failed to read object: {}", object_path.display()))?;

    // Try to decode as UTF-8 string
    String::from_utf8(content).with_context(|| {
        format!(
            "Object content is not valid UTF-8: {}",
            object_path.display()
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_conflict_marker_generation() {
        let local = "local content\n";
        let remote = "remote content\n";
        let branch = "feature";

        let marked = ConflictMarker::generate(local, remote, branch);

        assert!(marked.contains("<<<<<<< HEAD (local)"));
        assert!(marked.contains("local content"));
        assert!(marked.contains("======="));
        assert!(marked.contains("remote content"));
        assert!(marked.contains(">>>>>>> feature (remote)"));
    }

    #[test]
    fn test_has_conflict_markers() {
        let with_markers = "<<<<<<< HEAD\ncontent\n=======\nother\n>>>>>>>\n";
        let without_markers = "normal content\n";

        assert!(ConflictMarker::has_markers(with_markers));
        assert!(!ConflictMarker::has_markers(without_markers));
    }
}
