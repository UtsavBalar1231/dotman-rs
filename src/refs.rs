use crate::reflog::ReflogManager;
use anyhow::Result;
use std::fs;
use std::path::PathBuf;

pub mod resolver;

/// Manages git-like references (branches, HEAD, etc.)
pub struct RefManager {
    repo_path: PathBuf,
}

impl RefManager {
    pub fn new(repo_path: PathBuf) -> Self {
        Self { repo_path }
    }

    /// Initialize refs structure for a new repository
    pub fn init(&self) -> Result<()> {
        fs::create_dir_all(self.repo_path.join("refs/heads"))?;
        fs::create_dir_all(self.repo_path.join("refs/remotes"))?;
        fs::create_dir_all(self.repo_path.join("refs/tags"))?;

        self.create_branch("main", None)?;

        self.set_head_to_branch("main")?;

        Ok(())
    }

    /// Get the current branch name
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
    pub fn set_head_to_branch(&self, branch: &str) -> Result<()> {
        self.set_head_to_branch_with_reflog(
            branch,
            "checkout",
            &format!("checkout: moving to {}", branch),
        )
    }

    /// Set HEAD to point to a branch with reflog entry
    pub fn set_head_to_branch_with_reflog(
        &self,
        branch: &str,
        operation: &str,
        message: &str,
    ) -> Result<()> {
        let old_value = self
            .get_current_head_value()
            .unwrap_or_else(|| "0".repeat(40));

        let head_path = self.repo_path.join("HEAD");
        let new_ref = format!("ref: refs/heads/{}", branch);
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
    pub fn set_head_to_commit(&self, commit_id: &str) -> Result<()> {
        self.set_head_to_commit_with_reflog(
            commit_id,
            "checkout",
            &format!(
                "checkout: moving to {}",
                &commit_id[..8.min(commit_id.len())]
            ),
        )
    }

    /// Set HEAD to point directly to a commit with reflog entry
    pub fn set_head_to_commit_with_reflog(
        &self,
        commit_id: &str,
        operation: &str,
        message: &str,
    ) -> Result<()> {
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
    pub fn create_branch(&self, name: &str, commit_id: Option<&str>) -> Result<()> {
        let branch_path = self.repo_path.join(format!("refs/heads/{}", name));

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
    pub fn delete_branch(&self, name: &str) -> Result<()> {
        // Prevent deletion of current branch
        if self.current_branch()?.as_ref().is_some_and(|c| c == name) {
            anyhow::bail!("Cannot delete the current branch '{}'", name);
        }

        let branch_path = self.repo_path.join(format!("refs/heads/{}", name));
        if !branch_path.exists() {
            anyhow::bail!("Branch '{}' does not exist", name);
        }

        fs::remove_file(&branch_path)?;
        Ok(())
    }

    /// List all local branches
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
    pub fn get_branch_commit(&self, branch: &str) -> Result<String> {
        let branch_path = self.repo_path.join(format!("refs/heads/{}", branch));
        if !branch_path.exists() {
            anyhow::bail!("Branch '{}' does not exist", branch);
        }

        Ok(fs::read_to_string(&branch_path)?.trim().to_string())
    }

    /// Update the commit ID for a branch
    pub fn update_branch(&self, branch: &str, commit_id: &str) -> Result<()> {
        let branch_path = self.repo_path.join(format!("refs/heads/{}", branch));
        if !branch_path.exists() {
            anyhow::bail!("Branch '{}' does not exist", branch);
        }

        fs::write(&branch_path, commit_id)?;
        Ok(())
    }

    /// Get the current HEAD commit (whether from branch or detached)
    pub fn get_head_commit(&self) -> Result<Option<String>> {
        let head_path = self.repo_path.join("HEAD");
        if !head_path.exists() {
            return Ok(None);
        }

        let head_content = fs::read_to_string(&head_path)?.trim().to_string();

        if let Some(branch_name) = head_content.strip_prefix("ref: refs/heads/") {
            // Read the branch file to get the commit
            let branch_path = self.repo_path.join(format!("refs/heads/{}", branch_name));
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
    pub fn branch_exists(&self, name: &str) -> bool {
        self.repo_path.join(format!("refs/heads/{}", name)).exists()
    }

    /// Rename a branch
    pub fn rename_branch(&self, old_name: &str, new_name: &str) -> Result<()> {
        let old_path = self.repo_path.join(format!("refs/heads/{}", old_name));
        let new_path = self.repo_path.join(format!("refs/heads/{}", new_name));

        if !old_path.exists() {
            anyhow::bail!("Branch '{}' does not exist", old_name);
        }

        if new_path.exists() {
            anyhow::bail!("Branch '{}' already exists", new_name);
        }

        fs::rename(&old_path, &new_path)?;

        // Update HEAD if it pointed to the renamed branch
        if self
            .current_branch()?
            .as_ref()
            .is_some_and(|c| c == old_name)
        {
            self.set_head_to_branch_with_reflog(
                new_name,
                "branch",
                &format!("Branch: renamed {} to {}", old_name, new_name),
            )?;
        }

        Ok(())
    }

    // Tag management methods

    /// Create a new tag
    pub fn create_tag(&self, name: &str, commit_id: Option<&str>) -> Result<()> {
        let tag_path = self.repo_path.join(format!("refs/tags/{}", name));

        if tag_path.exists() {
            anyhow::bail!("Tag '{}' already exists", name);
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
    pub fn delete_tag(&self, name: &str) -> Result<()> {
        let tag_path = self.repo_path.join(format!("refs/tags/{}", name));

        if !tag_path.exists() {
            anyhow::bail!("Tag '{}' does not exist", name);
        }

        fs::remove_file(&tag_path)?;
        Ok(())
    }

    /// List all tags
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
    pub fn get_tag_commit(&self, tag: &str) -> Result<String> {
        let tag_path = self.repo_path.join(format!("refs/tags/{}", tag));
        if !tag_path.exists() {
            anyhow::bail!("Tag '{}' does not exist", tag);
        }

        Ok(fs::read_to_string(&tag_path)?.trim().to_string())
    }

    /// Check if a tag exists
    pub fn tag_exists(&self, name: &str) -> bool {
        self.repo_path.join(format!("refs/tags/{}", name)).exists()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn setup_test_repo() -> Result<(tempfile::TempDir, RefManager)> {
        let temp = tempdir()?;
        let repo_path = temp.path().join(".dotman");
        fs::create_dir_all(&repo_path)?;

        let manager = RefManager::new(repo_path);
        manager.init()?;

        Ok((temp, manager))
    }

    #[test]
    fn test_init_creates_structure() -> Result<()> {
        let (_temp, manager) = setup_test_repo()?;

        assert!(manager.repo_path.join("refs/heads").exists());
        assert!(manager.repo_path.join("refs/remotes").exists());
        assert!(manager.repo_path.join("HEAD").exists());
        assert!(manager.repo_path.join("refs/heads/main").exists());

        Ok(())
    }

    #[test]
    fn test_current_branch() -> Result<()> {
        let (_temp, manager) = setup_test_repo()?;

        let current = manager.current_branch()?;
        assert_eq!(current, Some("main".to_string()));

        Ok(())
    }

    #[test]
    fn test_create_and_list_branches() -> Result<()> {
        let (_temp, manager) = setup_test_repo()?;

        manager.create_branch("feature", None)?;
        manager.create_branch("bugfix", None)?;

        let branches = manager.list_branches()?;
        assert_eq!(branches.len(), 3);
        assert!(branches.contains(&"main".to_string()));
        assert!(branches.contains(&"feature".to_string()));
        assert!(branches.contains(&"bugfix".to_string()));

        Ok(())
    }

    #[test]
    fn test_delete_branch() -> Result<()> {
        let (_temp, manager) = setup_test_repo()?;

        manager.create_branch("temp", None)?;
        assert!(manager.branch_exists("temp"));

        manager.delete_branch("temp")?;
        assert!(!manager.branch_exists("temp"));

        Ok(())
    }

    #[test]
    fn test_cannot_delete_current_branch() -> Result<()> {
        let (_temp, manager) = setup_test_repo()?;

        let result = manager.delete_branch("main");
        assert!(result.is_err());

        Ok(())
    }

    #[test]
    fn test_rename_branch() -> Result<()> {
        let (_temp, manager) = setup_test_repo()?;

        manager.create_branch("old", None)?;
        manager.rename_branch("old", "new")?;

        assert!(!manager.branch_exists("old"));
        assert!(manager.branch_exists("new"));

        Ok(())
    }

    #[test]
    fn test_detached_head() -> Result<()> {
        let (_temp, manager) = setup_test_repo()?;

        manager.set_head_to_commit("abc123")?;

        let current = manager.current_branch()?;
        assert_eq!(current, None); // Detached HEAD

        let commit = manager.get_head_commit()?;
        assert_eq!(commit, Some("abc123".to_string()));

        Ok(())
    }
}
