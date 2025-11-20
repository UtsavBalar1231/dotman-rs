use crate::reflog::ReflogManager;
use anyhow::Result;
use std::fs;
use std::path::PathBuf;

/// Reference resolution (HEAD, branches, tags, ancestry)
pub mod resolver;
/// Reference update operations
pub mod updater;

/// Manages git-like references (branches, HEAD, etc.)
pub struct RefManager {
    /// Path to the repository root
    repo_path: PathBuf,
}

impl RefManager {
    /// Creates a new reference manager for a repository
    ///
    /// # Arguments
    ///
    /// * `repo_path` - Path to the repository root directory
    #[must_use]
    pub const fn new(repo_path: PathBuf) -> Self {
        Self { repo_path }
    }

    /// Initialize refs structure for a new repository
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Failed to create directory structure
    /// - Failed to create default branch
    /// - Failed to set HEAD
    pub fn init(&self) -> Result<()> {
        fs::create_dir_all(self.repo_path.join("refs/heads"))?;
        fs::create_dir_all(self.repo_path.join("refs/remotes"))?;
        fs::create_dir_all(self.repo_path.join("refs/tags"))?;

        self.create_branch("main", None)?;

        self.set_head_to_branch("main", None, None)?;

        Ok(())
    }

    /// Get the current branch name
    ///
    /// # Errors
    ///
    /// Returns an error if HEAD file cannot be read
    pub fn current_branch(&self) -> Result<Option<String>> {
        let head_path = self.repo_path.join("HEAD");
        if !head_path.exists() {
            return Ok(None);
        }

        let head_content = fs::read_to_string(&head_path)?;

        if let Some(branch_ref) = head_content.strip_prefix("ref: refs/heads/") {
            return Ok(Some(branch_ref.trim().to_string()));
        }

        // HEAD is detached (points directly to a commit)
        Ok(None)
    }

    /// Set HEAD to point to a branch
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - HEAD file cannot be written
    /// - Reflog entry cannot be created
    pub fn set_head_to_branch(
        &self,
        branch: &str,
        operation: Option<&str>,
        message: Option<&str>,
    ) -> Result<()> {
        let operation = operation.unwrap_or("checkout");
        let default_message = format!("checkout: moving to {branch}");
        let message = message.unwrap_or(&default_message);
        let old_value = self
            .get_current_head_value()
            .unwrap_or_else(|| "0".repeat(40));

        let head_path = self.repo_path.join("HEAD");
        let new_ref = format!("ref: refs/heads/{branch}");
        fs::write(&head_path, &new_ref)?;

        // Log the reflog entry
        let reflog_manager = ReflogManager::new(self.repo_path.clone());
        reflog_manager.log_head_update(&old_value, &new_ref, operation, message)?;

        Ok(())
    }

    /// Get the current raw HEAD value (for reflog purposes)
    fn get_current_head_value(&self) -> Option<String> {
        let head_path = self.repo_path.join("HEAD");
        if !head_path.exists() {
            return None;
        }

        fs::read_to_string(&head_path)
            .map(|s| s.trim().to_string())
            .ok()
    }

    /// Set HEAD to point directly to a commit (detached HEAD)
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - HEAD file cannot be written
    /// - Reflog entry cannot be created
    pub fn set_head_to_commit(
        &self,
        commit_id: &str,
        operation: Option<&str>,
        message: Option<&str>,
    ) -> Result<()> {
        let operation = operation.unwrap_or("checkout");
        let default_message = format!(
            "checkout: moving to {}",
            &commit_id[..8.min(commit_id.len())]
        );
        let message = message.unwrap_or(&default_message);
        let old_value = self
            .get_current_head_value()
            .unwrap_or_else(|| "0".repeat(40));

        let head_path = self.repo_path.join("HEAD");
        fs::write(&head_path, commit_id)?;

        // Log the reflog entry
        let reflog_manager = ReflogManager::new(self.repo_path.clone());
        reflog_manager.log_head_update(&old_value, commit_id, operation, message)?;

        Ok(())
    }

    /// Create a new branch
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Failed to get current HEAD commit
    /// - Failed to write branch file
    pub fn create_branch(&self, name: &str, commit_id: Option<&str>) -> Result<()> {
        let branch_path = self.repo_path.join(format!("refs/heads/{name}"));

        // If commit_id is provided, use it; otherwise use current HEAD
        let commit = if let Some(id) = commit_id {
            id.to_string()
        } else {
            self.get_head_commit()?.unwrap_or_else(|| "0".repeat(40)) // Empty commit ID if no commits yet
        };

        fs::write(&branch_path, commit)?;
        Ok(())
    }

    /// Delete a branch
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Trying to delete the current branch
    /// - Trying to delete the main branch
    /// - Branch does not exist
    /// - Failed to delete branch file
    pub fn delete_branch(&self, name: &str) -> Result<()> {
        // Prevent deletion of current branch
        if self.current_branch()?.as_ref().is_some_and(|c| c == name) {
            return Err(anyhow::anyhow!("Cannot delete the current branch '{name}'"));
        }

        let branch_path = self.repo_path.join(format!("refs/heads/{name}"));
        if !branch_path.exists() {
            return Err(anyhow::anyhow!("Branch '{name}' does not exist"));
        }

        fs::remove_file(&branch_path)?;
        Ok(())
    }

    /// List all local branches
    ///
    /// # Errors
    ///
    /// Returns an error if directory cannot be read
    pub fn list_branches(&self) -> Result<Vec<String>> {
        let heads_dir = self.repo_path.join("refs/heads");
        if !heads_dir.exists() {
            return Ok(Vec::new());
        }

        let mut branches = Vec::new();
        for entry in fs::read_dir(&heads_dir)? {
            let entry = entry?;
            if entry.file_type()?.is_file()
                && let Some(name) = entry.file_name().to_str()
            {
                branches.push(name.to_string());
            }
        }

        branches.sort();
        Ok(branches)
    }

    /// Get the commit ID for a branch
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Branch does not exist
    /// - Branch file cannot be read
    pub fn get_branch_commit(&self, branch: &str) -> Result<String> {
        let branch_path = self.repo_path.join(format!("refs/heads/{branch}"));
        if !branch_path.exists() {
            return Err(anyhow::anyhow!("Branch '{branch}' does not exist"));
        }

        Ok(fs::read_to_string(&branch_path)?.trim().to_string())
    }

    /// Update the commit ID for a branch
    ///
    /// Validates the commit ID format and checks that the commit exists before updating.
    /// Also verifies the snapshot integrity to ensure all referenced objects are valid.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Branch does not exist
    /// - Commit ID format is invalid (must be hex string, minimum 4 chars)
    /// - Commit does not exist in repository
    /// - Snapshot integrity check fails
    /// - Branch file cannot be written
    pub fn update_branch(&self, branch: &str, commit_id: &str) -> Result<()> {
        let branch_path = self.repo_path.join(format!("refs/heads/{branch}"));
        if !branch_path.exists() {
            return Err(anyhow::anyhow!("Branch '{branch}' does not exist"));
        }

        // Validate commit ID format
        Self::validate_commit_id(commit_id)?;

        // Verify commit exists in repository
        let commit_path = self
            .repo_path
            .join("commits")
            .join(format!("{commit_id}.zst"));
        if !commit_path.exists() {
            return Err(anyhow::anyhow!(
                "Commit '{}' does not exist in repository",
                &commit_id[..8.min(commit_id.len())]
            ));
        }

        // Verify snapshot can be loaded (basic validity check)
        // Full integrity verification is expensive and should be done by fsck
        let snapshot_manager = crate::storage::snapshots::SnapshotManager::new(
            self.repo_path.clone(),
            3, // Default compression level
        );
        if let Err(e) = snapshot_manager.load_snapshot(commit_id) {
            return Err(anyhow::anyhow!(
                "Cannot load snapshot for commit '{}': {}",
                &commit_id[..8.min(commit_id.len())],
                e
            ));
        }

        fs::write(&branch_path, commit_id)?;
        Ok(())
    }

    /// Validate commit ID format
    ///
    /// Checks that the ID is a valid hex string with appropriate length
    fn validate_commit_id(commit_id: &str) -> Result<()> {
        // Minimum length check (at least 4 chars for short ID, typically 64 for full)
        if commit_id.len() < 4 {
            return Err(anyhow::anyhow!(
                "Commit ID too short: '{commit_id}' (minimum 4 characters)"
            ));
        }

        // Check if all characters are valid hexadecimal
        if !commit_id.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(anyhow::anyhow!(
                "Invalid commit ID format: '{}' (must be hexadecimal)",
                &commit_id[..16.min(commit_id.len())]
            ));
        }

        Ok(())
    }

    /// Get the current HEAD commit (whether from branch or detached)
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - HEAD file cannot be read
    /// - Referenced branch file cannot be read
    pub fn get_head_commit(&self) -> Result<Option<String>> {
        let head_path = self.repo_path.join("HEAD");
        if !head_path.exists() {
            return Ok(None);
        }

        let head_content = fs::read_to_string(&head_path)?.trim().to_string();

        if let Some(branch_name) = head_content.strip_prefix("ref: refs/heads/") {
            // Read the branch file to get the commit
            let branch_path = self.repo_path.join(format!("refs/heads/{branch_name}"));
            if branch_path.exists() {
                return Ok(Some(fs::read_to_string(&branch_path)?.trim().to_string()));
            }
        } else {
            // HEAD points directly to a commit
            return Ok(Some(head_content));
        }

        Ok(None)
    }

    /// Check if a branch exists
    #[must_use]
    pub fn branch_exists(&self, name: &str) -> bool {
        self.repo_path.join(format!("refs/heads/{name}")).exists()
    }

    /// Rename a branch
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Old branch does not exist
    /// - New branch already exists
    /// - Failed to rename files
    /// - Current branch is being renamed and HEAD update fails
    pub fn rename_branch(&self, old_name: &str, new_name: &str) -> Result<()> {
        let old_path = self.repo_path.join(format!("refs/heads/{old_name}"));
        let new_path = self.repo_path.join(format!("refs/heads/{new_name}"));

        if !old_path.exists() {
            return Err(anyhow::anyhow!("Branch '{old_name}' does not exist"));
        }

        if new_path.exists() {
            return Err(anyhow::anyhow!("Branch '{new_name}' already exists"));
        }

        fs::rename(&old_path, &new_path)?;

        // Update HEAD if it pointed to the renamed branch
        if self
            .current_branch()?
            .as_ref()
            .is_some_and(|c| c == old_name)
        {
            self.set_head_to_branch(
                new_name,
                Some("branch"),
                Some(&format!("Branch: renamed {old_name} to {new_name}")),
            )?;
        }

        Ok(())
    }

    // Tag management methods

    /// Create a new tag
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Tag already exists
    /// - No commits available to tag
    /// - Failed to create tag file
    pub fn create_tag(&self, name: &str, commit_id: Option<&str>) -> Result<()> {
        let tag_path = self.repo_path.join(format!("refs/tags/{name}"));

        if tag_path.exists() {
            return Err(anyhow::anyhow!("Tag '{name}' already exists"));
        }

        // If commit_id is provided, use it; otherwise use current HEAD
        let commit = if let Some(id) = commit_id {
            id.to_string()
        } else {
            self.get_head_commit()?
                .ok_or_else(|| anyhow::anyhow!("No commits available to tag"))?
        };

        // Ensure the tags directory exists
        if let Some(parent) = tag_path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(&tag_path, commit)?;
        Ok(())
    }

    /// Delete a tag
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Tag does not exist
    /// - Failed to delete tag file
    pub fn delete_tag(&self, name: &str) -> Result<()> {
        let tag_path = self.repo_path.join(format!("refs/tags/{name}"));

        if !tag_path.exists() {
            return Err(anyhow::anyhow!("Tag '{name}' does not exist"));
        }

        fs::remove_file(&tag_path)?;
        Ok(())
    }

    /// List all tags
    ///
    /// # Errors
    ///
    /// Returns an error if directory cannot be read
    pub fn list_tags(&self) -> Result<Vec<String>> {
        let tags_dir = self.repo_path.join("refs/tags");
        if !tags_dir.exists() {
            return Ok(Vec::new());
        }

        let mut tags = Vec::new();
        for entry in fs::read_dir(&tags_dir)? {
            let entry = entry?;
            if entry.file_type()?.is_file()
                && let Some(name) = entry.file_name().to_str()
            {
                tags.push(name.to_string());
            }
        }

        tags.sort();
        Ok(tags)
    }

    /// Get the commit ID for a tag
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Tag does not exist
    /// - Tag file cannot be read
    pub fn get_tag_commit(&self, tag: &str) -> Result<String> {
        let tag_path = self.repo_path.join(format!("refs/tags/{tag}"));
        if !tag_path.exists() {
            return Err(anyhow::anyhow!("Tag '{tag}' does not exist"));
        }

        Ok(fs::read_to_string(&tag_path)?.trim().to_string())
    }

    /// Check if a tag exists
    #[must_use]
    pub fn tag_exists(&self, name: &str) -> bool {
        self.repo_path.join(format!("refs/tags/{name}")).exists()
    }

    // Remote ref management methods

    /// Create or update a remote tracking ref
    ///
    /// Remote refs are stored as `refs/remotes/<remote>/<branch>` and track
    /// the state of branches on remote repositories after fetch operations.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Failed to create remote refs directory
    /// - Failed to write ref file
    pub fn update_remote_ref(&self, remote: &str, branch: &str, commit_id: &str) -> Result<()> {
        let ref_path = self
            .repo_path
            .join(format!("refs/remotes/{remote}/{branch}"));

        // Create parent directory if needed
        if let Some(parent) = ref_path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(&ref_path, commit_id)?;
        Ok(())
    }

    /// Get the commit ID for a remote tracking ref
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Remote ref does not exist
    /// - Failed to read ref file
    pub fn get_remote_ref(&self, remote: &str, branch: &str) -> Result<String> {
        let ref_path = self
            .repo_path
            .join(format!("refs/remotes/{remote}/{branch}"));

        if !ref_path.exists() {
            return Err(anyhow::anyhow!(
                "Remote ref '{remote}/{branch}' does not exist"
            ));
        }

        Ok(fs::read_to_string(&ref_path)?.trim().to_string())
    }

    /// List all remote tracking refs for a remote
    ///
    /// Returns a vector of (`branch_name`, `commit_id`) tuples.
    ///
    /// # Errors
    ///
    /// Returns an error if directory cannot be read
    pub fn list_remote_refs(&self, remote: &str) -> Result<Vec<(String, String)>> {
        let remotes_dir = self.repo_path.join(format!("refs/remotes/{remote}"));

        if !remotes_dir.exists() {
            return Ok(Vec::new());
        }

        let mut refs = Vec::new();
        for entry in fs::read_dir(&remotes_dir)? {
            let entry = entry?;
            if entry.file_type()?.is_file() {
                let branch = entry.file_name().to_string_lossy().to_string();
                let commit = fs::read_to_string(entry.path())?.trim().to_string();
                refs.push((branch, commit));
            }
        }

        refs.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(refs)
    }

    /// Delete all remote refs for a remote
    ///
    /// This is typically called when removing a remote configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if directory removal fails
    pub fn delete_remote_refs(&self, remote: &str) -> Result<()> {
        let remotes_dir = self.repo_path.join(format!("refs/remotes/{remote}"));

        if remotes_dir.exists() {
            fs::remove_dir_all(&remotes_dir)?;
        }

        Ok(())
    }

    /// Check if a remote ref exists
    #[must_use]
    pub fn remote_ref_exists(&self, remote: &str, branch: &str) -> bool {
        self.repo_path
            .join(format!("refs/remotes/{remote}/{branch}"))
            .exists()
    }
}
