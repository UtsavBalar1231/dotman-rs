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
            return Err(anyhow::anyhow!(
                "Cannot delete the current branch '{}'",
                name
            ));
        }

        let branch_path = self.repo_path.join(format!("refs/heads/{name}"));
        if !branch_path.exists() {
            return Err(anyhow::anyhow!("Branch '{}' does not exist", name));
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
            return Err(anyhow::anyhow!("Branch '{}' does not exist", branch));
        }

        Ok(fs::read_to_string(&branch_path)?.trim().to_string())
    }

    /// Update the commit ID for a branch
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Branch does not exist
    /// - Branch file cannot be written
    pub fn update_branch(&self, branch: &str, commit_id: &str) -> Result<()> {
        let branch_path = self.repo_path.join(format!("refs/heads/{branch}"));
        if !branch_path.exists() {
            return Err(anyhow::anyhow!("Branch '{}' does not exist", branch));
        }

        fs::write(&branch_path, commit_id)?;
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
            return Err(anyhow::anyhow!("Branch '{}' does not exist", old_name));
        }

        if new_path.exists() {
            return Err(anyhow::anyhow!("Branch '{}' already exists", new_name));
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
            return Err(anyhow::anyhow!("Tag '{}' already exists", name));
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
            return Err(anyhow::anyhow!("Tag '{}' does not exist", name));
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
            return Err(anyhow::anyhow!("Tag '{}' does not exist", tag));
        }

        Ok(fs::read_to_string(&tag_path)?.trim().to_string())
    }

    /// Check if a tag exists
    #[must_use]
    pub fn tag_exists(&self, name: &str) -> bool {
        self.repo_path.join(format!("refs/tags/{name}")).exists()
    }
}
