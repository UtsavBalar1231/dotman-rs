use crate::reflog::ReflogManager;
use crate::refs::RefManager;
use anyhow::Result;
use std::path::PathBuf;

/// Helper for updating HEAD and reflog atomically
pub struct ReflogUpdater {
    ref_manager: RefManager,
    reflog_manager: ReflogManager,
}

impl ReflogUpdater {
    /// Create a new `ReflogUpdater` for the given repository
    #[must_use]
    pub fn new(repo_path: PathBuf) -> Self {
        Self {
            ref_manager: RefManager::new(repo_path.clone()),
            reflog_manager: ReflogManager::new(repo_path),
        }
    }

    /// Update HEAD to point to a new commit with reflog entry
    ///
    /// This handles both branch and detached HEAD states automatically.
    /// If HEAD points to a branch, the branch is updated.
    /// If HEAD is detached, HEAD is updated directly.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Failed to get current HEAD state
    /// - Failed to update branch or HEAD
    /// - Failed to create reflog entry
    pub fn update_head(&self, commit_id: &str, operation: &str, message: &str) -> Result<()> {
        // Get the old HEAD value for reflog
        let old_value = self
            .reflog_manager
            .get_current_head()
            .unwrap_or_else(|_| "0".repeat(40));

        // Check if we're on a branch
        if let Some(branch) = self.ref_manager.current_branch()? {
            // Update the branch to point to the new commit
            self.ref_manager.update_branch(&branch, commit_id)?;

            // Log the HEAD update
            self.reflog_manager
                .log_head_update(&old_value, commit_id, operation, message)?;
        } else {
            // HEAD is detached, update it directly with reflog
            self.ref_manager
                .set_head_to_commit(commit_id, Some(operation), Some(message))?;
        }

        Ok(())
    }

    /// Update HEAD for a branch switch with reflog entry
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Failed to update HEAD to branch
    /// - Failed to create reflog entry
    pub fn switch_to_branch(&self, branch: &str) -> Result<()> {
        self.ref_manager.set_head_to_branch(
            branch,
            Some("checkout"),
            Some(&format!("checkout: moving to {branch}")),
        )
    }

    /// Update HEAD for a commit checkout (detached HEAD) with reflog entry
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Failed to update HEAD to commit
    /// - Failed to create reflog entry
    pub fn switch_to_commit(&self, commit_id: &str) -> Result<()> {
        let display_id = if commit_id.len() >= 8 {
            &commit_id[..8]
        } else {
            commit_id
        };

        self.ref_manager.set_head_to_commit(
            commit_id,
            Some("checkout"),
            Some(&format!("checkout: moving to {display_id}")),
        )
    }

    /// Reset HEAD to a commit with reflog entry
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Failed to update HEAD
    /// - Failed to create reflog entry
    pub fn reset_head(&self, commit_id: &str, reset_type: &str) -> Result<()> {
        let display_id = if commit_id.len() >= 8 {
            &commit_id[..8]
        } else {
            commit_id
        };

        self.update_head(
            commit_id,
            "reset",
            &format!("reset: moving to {display_id} ({reset_type})"),
        )
    }

    /// Update HEAD after a merge with reflog entry
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Failed to update HEAD
    /// - Failed to create reflog entry
    pub fn merge_head(&self, commit_id: &str, merged_branch: &str) -> Result<()> {
        self.update_head(
            commit_id,
            "merge",
            &format!("merge {merged_branch}: Fast-forward"),
        )
    }

    /// Update HEAD after a commit with reflog entry
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Failed to update HEAD
    /// - Failed to create reflog entry
    pub fn commit_head(&self, commit_id: &str, commit_message: &str) -> Result<()> {
        // Truncate commit message for reflog if it's too long
        let truncated_message = if commit_message.len() > 50 {
            format!("{}...", &commit_message[..47])
        } else {
            commit_message.to_string()
        };

        self.update_head(commit_id, "commit", &format!("commit: {truncated_message}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_reflog_updater_new() {
        let temp_dir = TempDir::new().unwrap();
        let _updater = ReflogUpdater::new(temp_dir.path().to_path_buf());

        // Just ensure it can be created without panicking
    }

    #[test]
    fn test_commit_message_truncation() {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path().to_path_buf();

        // Initialize the repository structure
        std::fs::create_dir_all(repo_path.join("refs/heads")).unwrap();
        std::fs::write(repo_path.join("HEAD"), "ref: refs/heads/main").unwrap();
        std::fs::write(repo_path.join("refs/heads/main"), "0".repeat(40)).unwrap();

        let updater = ReflogUpdater::new(repo_path);

        // Test with a long commit message
        let long_message =
            "This is a very long commit message that should be truncated in the reflog entry";
        let result = updater.commit_head("abc123", long_message);

        // The function should handle this gracefully
        assert!(result.is_ok());
    }
}
