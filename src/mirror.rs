use crate::config::Config;
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Manages git mirror repositories for remote synchronization
pub struct GitMirror {
    /// Path to the mirror repository (.dotman/mirrors/{remote-name})
    mirror_path: PathBuf,
    /// Name of the remote
    #[allow(dead_code)]
    remote_name: String,
    /// URL of the remote repository
    remote_url: String,
    /// Configuration
    config: Config,
}

impl GitMirror {
    /// Create a new `GitMirror` instance
    #[must_use]
    pub fn new(repo_path: &Path, remote_name: &str, remote_url: &str, config: Config) -> Self {
        let mirror_path = repo_path.join("mirrors").join(remote_name);
        Self {
            mirror_path,
            remote_name: remote_name.to_string(),
            remote_url: remote_url.to_string(),
            config,
        }
    }

    /// Initialize the mirror repository if it doesn't exist
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Failed to create mirror directory
    /// - Git init command fails
    /// - Failed to configure git user
    /// - Failed to add remote
    pub fn init_mirror(&self) -> Result<()> {
        if self.mirror_path.exists() {
            // Ensure remote is configured correctly
            self.update_remote()?;
        } else {
            fs::create_dir_all(&self.mirror_path).context("Failed to create mirror directory")?;

            let output = Command::new("git")
                .args(["init"])
                .current_dir(&self.mirror_path)
                .output()
                .context("Failed to initialize git repository")?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(anyhow::anyhow!("Git init failed: {stderr}"));
            }

            // Configure git user for the repository (required for commits)
            // Use dotman config if available, otherwise use defaults
            let user_email = self
                .config
                .user
                .email
                .as_deref()
                .unwrap_or("dotman@localhost");
            let user_name = self.config.user.name.as_deref().unwrap_or("Dotman");

            Command::new("git")
                .args(["config", "user.email", user_email])
                .current_dir(&self.mirror_path)
                .output()
                .context("Failed to configure git email")?;

            Command::new("git")
                .args(["config", "user.name", user_name])
                .current_dir(&self.mirror_path)
                .output()
                .context("Failed to configure git name")?;

            // Add remote
            self.add_remote()?;
        }

        Ok(())
    }

    /// Add the remote to the mirror repository
    fn add_remote(&self) -> Result<()> {
        let output = Command::new("git")
            .args(["remote", "add", "origin", &self.remote_url])
            .current_dir(&self.mirror_path)
            .output()
            .context("Failed to add git remote")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // Ignore if remote already exists
            if !stderr.contains("already exists") {
                return Err(anyhow::anyhow!("Git remote add failed: {stderr}"));
            }
        }

        Ok(())
    }

    /// Update the remote URL if it has changed
    fn update_remote(&self) -> Result<()> {
        let output = Command::new("git")
            .args(["remote", "set-url", "origin", &self.remote_url])
            .current_dir(&self.mirror_path)
            .output()
            .context("Failed to update git remote")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("Git remote set-url failed: {stderr}"));
        }

        Ok(())
    }

    /// Sync files from dotman storage to the mirror repository
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Failed to create parent directories
    /// - Failed to copy files
    pub fn sync_from_dotman(&self, files: &[(PathBuf, PathBuf)]) -> Result<()> {
        // files is a list of (source_path, relative_path) tuples
        for (source_path, relative_path) in files {
            let dest_path = self.mirror_path.join(relative_path);

            // Create parent directories if needed
            if let Some(parent) = dest_path.parent() {
                fs::create_dir_all(parent).context("Failed to create parent directories")?;
            }

            // Copy the file
            if source_path.exists() {
                fs::copy(source_path, &dest_path).with_context(|| {
                    format!("Failed to copy {} to mirror", source_path.display())
                })?;

                // Preserve file permissions
                #[cfg(unix)]
                {
                    let metadata = fs::metadata(source_path)?;
                    let permissions = metadata.permissions();
                    fs::set_permissions(&dest_path, permissions)?;
                }
            }
        }

        Ok(())
    }

    /// Add all changes and commit in the mirror repository
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Git add command fails
    /// - Git commit command fails
    /// - Failed to get HEAD commit
    pub fn commit(&self, message: &str, author: &str) -> Result<String> {
        // Add all changes
        let output = Command::new("git")
            .args(["add", "-A"])
            .current_dir(&self.mirror_path)
            .output()
            .context("Failed to add files to git")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("Git add failed: {stderr}"));
        }

        let output = Command::new("git")
            .args(["status", "--porcelain"])
            .current_dir(&self.mirror_path)
            .output()
            .context("Failed to check git status")?;

        if output.stdout.is_empty() {
            // No changes to commit, get current commit ID
            return self.get_head_commit();
        }

        // Format author as "Name <email@example.com>" for git
        let formatted_author = if author.contains('<') && author.contains('>') {
            author.to_string()
        } else {
            format!(
                "{} <{}@dotman.local>",
                author,
                author.to_lowercase().replace(' ', ".")
            )
        };

        // Commit changes
        let output = Command::new("git")
            .args(["commit", "-m", message, "--author", &formatted_author])
            .current_dir(&self.mirror_path)
            .output()
            .context("Failed to commit changes")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("Git commit failed: {stderr}"));
        }

        self.get_head_commit()
    }

    /// Add all changes and commit with a specific timestamp
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Git add command fails
    /// - Invalid timestamp provided
    /// - Git commit command fails
    /// - Failed to get HEAD commit
    pub fn commit_with_timestamp(
        &self,
        message: &str,
        author: &str,
        timestamp: i64,
    ) -> Result<String> {
        use chrono::{TimeZone, Utc};

        // Add all changes
        let output = Command::new("git")
            .args(["add", "-A"])
            .current_dir(&self.mirror_path)
            .output()
            .context("Failed to add files to git")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("Git add failed: {stderr}"));
        }

        let output = Command::new("git")
            .args(["status", "--porcelain"])
            .current_dir(&self.mirror_path)
            .output()
            .context("Failed to check git status")?;

        if output.stdout.is_empty() {
            // No changes to commit, get current commit ID
            return self.get_head_commit();
        }

        // Format author as "Name <email@example.com>" for git
        let formatted_author = if author.contains('<') && author.contains('>') {
            author.to_string()
        } else {
            format!(
                "{} <{}@dotman.local>",
                author,
                author.to_lowercase().replace(' ', ".")
            )
        };

        // Format timestamp for git (ISO 8601 format)
        let dt = Utc
            .timestamp_opt(timestamp, 0)
            .single()
            .context("Invalid timestamp")?;
        let date_str = dt.format("%Y-%m-%d %H:%M:%S %z").to_string();

        // Commit changes with specific date
        let output = Command::new("git")
            .args(["commit", "-m", message, "--author", &formatted_author])
            .env("GIT_AUTHOR_DATE", &date_str)
            .env("GIT_COMMITTER_DATE", &date_str)
            .current_dir(&self.mirror_path)
            .output()
            .context("Failed to commit changes")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("Git commit failed: {stderr}"));
        }

        self.get_head_commit()
    }

    /// Clear all files from the working directory (but keep .git)
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Failed to remove tracked files
    /// - Failed to read or delete directory entries
    pub fn clear_working_directory(&self) -> Result<()> {
        let _output = Command::new("git")
            .args(["rm", "-rf", "--cached", "."])
            .current_dir(&self.mirror_path)
            .output()
            .context("Failed to remove tracked files")?;

        // Also physically remove the files (except .git)
        for entry in std::fs::read_dir(&self.mirror_path)? {
            let entry = entry?;
            let path = entry.path();
            let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

            // Skip .git directory
            if file_name != ".git" {
                if path.is_dir() {
                    std::fs::remove_dir_all(&path)?;
                } else {
                    std::fs::remove_file(&path)?;
                }
            }
        }

        Ok(())
    }

    /// Push changes to the remote repository
    ///
    /// # Errors
    ///
    /// Returns an error if the push fails
    pub fn push(&self, branch: &str) -> Result<()> {
        self.push_with_options(branch, false, false)
    }

    /// Push changes with force options
    ///
    /// # Errors
    ///
    /// Returns an error if the git push command fails
    pub fn push_with_options(
        &self,
        branch: &str,
        force: bool,
        force_with_lease: bool,
    ) -> Result<()> {
        // First try to fetch to see if remote exists
        let _ = Command::new("git")
            .args(["fetch", "origin"])
            .current_dir(&self.mirror_path)
            .output();

        // Build push command arguments
        let mut args = vec!["push", "origin", branch];

        if force {
            args.push("--force");
        } else if force_with_lease {
            args.push("--force-with-lease");
        }

        // Push to remote
        let output = Command::new("git")
            .args(&args)
            .current_dir(&self.mirror_path)
            .output()
            .context("Failed to push to remote")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);

            // Try with --set-upstream if branch doesn't exist on remote and not forcing
            if !force
                && !force_with_lease
                && (stderr.contains("has no upstream branch") || stderr.contains("src refspec"))
            {
                let output = Command::new("git")
                    .args(["push", "--set-upstream", "origin", branch])
                    .current_dir(&self.mirror_path)
                    .output()
                    .context("Failed to push with --set-upstream")?;

                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    return Err(anyhow::anyhow!("Git push failed: {stderr}"));
                }
            } else {
                return Err(anyhow::anyhow!("Git push failed: {stderr}"));
            }
        }

        Ok(())
    }

    /// Fetch changes from remote without merging
    ///
    /// # Errors
    ///
    /// Returns an error if the git fetch command fails
    pub fn fetch(&self, branch: Option<&str>) -> Result<()> {
        let mut args = vec!["fetch", "origin"];

        if let Some(b) = branch {
            args.push(b);
        }

        let output = Command::new("git")
            .args(&args)
            .current_dir(&self.mirror_path)
            .output()
            .context("Failed to fetch from remote")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("Git fetch failed: {stderr}"));
        }

        Ok(())
    }

    /// Merge a branch into the current branch
    ///
    /// # Errors
    ///
    /// Returns an error if the git merge command fails
    pub fn merge(&self, branch: &str, no_ff: bool) -> Result<()> {
        let mut args = vec!["merge", branch];

        if no_ff {
            args.push("--no-ff");
        }

        let output = Command::new("git")
            .args(&args)
            .current_dir(&self.mirror_path)
            .output()
            .context("Failed to merge branch")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("Git merge failed: {stderr}"));
        }

        Ok(())
    }

    /// Push tags to remote
    ///
    /// # Errors
    ///
    /// Returns an error if git push fails or tags cannot be pushed
    pub fn push_tags(&self) -> Result<()> {
        let output = Command::new("git")
            .args(["push", "origin", "--tags"])
            .current_dir(&self.mirror_path)
            .output()
            .context("Failed to push tags")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("Git push tags failed: {stderr}"));
        }

        Ok(())
    }

    /// Pull changes from the remote repository
    ///
    /// # Errors
    ///
    /// Returns an error if git fetch or merge fails
    pub fn pull(&self, branch: &str) -> Result<()> {
        // Fetch from remote
        let output = Command::new("git")
            .args(["fetch", "origin"])
            .current_dir(&self.mirror_path)
            .output()
            .context("Failed to fetch from remote")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("Git fetch failed: {stderr}"));
        }

        let output = Command::new("git")
            .args(["rev-parse", "--verify", branch])
            .current_dir(&self.mirror_path)
            .output()?;

        if output.status.success() {
            // Branch exists, checkout and pull
            let output = Command::new("git")
                .args(["checkout", branch])
                .current_dir(&self.mirror_path)
                .output()
                .context("Failed to checkout branch")?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(anyhow::anyhow!("Git checkout failed: {stderr}"));
            }

            // Pull changes
            let output = Command::new("git")
                .args(["pull", "origin", branch])
                .current_dir(&self.mirror_path)
                .output()
                .context("Failed to pull from remote")?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(anyhow::anyhow!("Git pull failed: {stderr}"));
            }
        } else {
            // Branch doesn't exist locally, create it from remote
            let output = Command::new("git")
                .args(["checkout", "-b", branch, &format!("origin/{branch}")])
                .current_dir(&self.mirror_path)
                .output()
                .context("Failed to create local branch")?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(anyhow::anyhow!("Git checkout failed: {stderr}"));
            }
        }

        Ok(())
    }

    /// Get the current HEAD commit ID
    ///
    /// # Errors
    ///
    /// Returns an error if git rev-parse fails or HEAD is not found
    pub fn get_head_commit(&self) -> Result<String> {
        let output = Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(&self.mirror_path)
            .output()
            .context("Failed to get HEAD commit")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("Git rev-parse failed: {stderr}"));
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    /// Get the path to the mirror repository
    #[must_use]
    pub fn get_mirror_path(&self) -> &Path {
        &self.mirror_path
    }

    /// List all files in the mirror repository
    ///
    /// # Errors
    ///
    /// Returns an error if git ls-files fails
    pub fn list_files(&self) -> Result<Vec<PathBuf>> {
        let output = Command::new("git")
            .args(["ls-files"])
            .current_dir(&self.mirror_path)
            .output()
            .context("Failed to list files in mirror")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("Git ls-files failed: {stderr}"));
        }

        let files = String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(PathBuf::from)
            .collect();

        Ok(files)
    }

    /// Checkout a specific branch in the mirror
    ///
    /// # Errors
    ///
    /// Returns an error if git checkout fails or branch cannot be created
    pub fn checkout_branch(&self, branch: &str) -> Result<()> {
        let output = Command::new("git")
            .args(["checkout", branch])
            .current_dir(&self.mirror_path)
            .output()
            .context("Failed to checkout branch")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);

            // Try creating the branch if it doesn't exist
            if stderr.contains("did not match any file") {
                let output = Command::new("git")
                    .args(["checkout", "-b", branch])
                    .current_dir(&self.mirror_path)
                    .output()
                    .context("Failed to create branch")?;

                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    return Err(anyhow::anyhow!("Git checkout -b failed: {stderr}"));
                }
            } else {
                return Err(anyhow::anyhow!("Git checkout failed: {stderr}"));
            }
        }

        Ok(())
    }

    /// Remove files from the mirror that are not in the provided list
    ///
    /// # Errors
    ///
    /// Returns an error if file listing or removal fails
    pub fn clean_removed_files(&self, current_files: &[PathBuf]) -> Result<()> {
        let mirror_files = self.list_files()?;

        for mirror_file in &mirror_files {
            if !current_files.contains(mirror_file) {
                let file_path = self.mirror_path.join(mirror_file);
                if file_path.exists() {
                    fs::remove_file(&file_path)
                        .with_context(|| format!("Failed to remove {}", file_path.display()))?;
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_mirror_creation() -> Result<()> {
        let temp = tempdir()?;
        let repo_path = temp.path().join(".dotman");
        fs::create_dir_all(&repo_path)?;

        let config = crate::config::Config::default();
        let mirror = GitMirror::new(
            &repo_path,
            "origin",
            "https://github.com/user/repo.git",
            config,
        );
        mirror.init_mirror()?;

        assert!(mirror.get_mirror_path().exists());
        assert!(mirror.get_mirror_path().join(".git").exists());

        Ok(())
    }

    #[test]
    fn test_sync_from_dotman() -> Result<()> {
        let temp = tempdir()?;
        let repo_path = temp.path().join(".dotman");
        fs::create_dir_all(&repo_path)?;

        let source_file = temp.path().join("source.txt");
        fs::write(&source_file, "test content")?;

        let config = crate::config::Config::default();
        let mirror = GitMirror::new(
            &repo_path,
            "origin",
            "https://github.com/user/repo.git",
            config,
        );
        mirror.init_mirror()?;

        let files = vec![(source_file, PathBuf::from("dest.txt"))];
        mirror.sync_from_dotman(&files)?;

        let dest_file = mirror.get_mirror_path().join("dest.txt");
        assert!(dest_file.exists());
        assert_eq!(fs::read_to_string(&dest_file)?, "test content");

        Ok(())
    }

    #[test]
    fn test_commit_with_changes() -> Result<()> {
        let temp = tempdir()?;
        let repo_path = temp.path().join(".dotman");
        fs::create_dir_all(&repo_path)?;

        let config = crate::config::Config::default();
        let mirror = GitMirror::new(
            &repo_path,
            "origin",
            "https://github.com/user/repo.git",
            config,
        );
        mirror.init_mirror()?;

        let test_file = mirror.get_mirror_path().join("test.txt");
        fs::write(&test_file, "test content")?;

        // Commit changes
        let commit_id = mirror.commit("Test commit", "Test User")?;
        assert!(!commit_id.is_empty());
        assert_eq!(commit_id.len(), 40); // Git SHA-1 is 40 chars

        // Verify file is tracked
        let files = mirror.list_files()?;
        assert_eq!(files.len(), 1);
        assert_eq!(files[0], PathBuf::from("test.txt"));

        Ok(())
    }

    #[test]
    fn test_checkout_branch() -> Result<()> {
        let temp = tempdir()?;
        let repo_path = temp.path().join(".dotman");
        fs::create_dir_all(&repo_path)?;

        let config = crate::config::Config::default();
        let mirror = GitMirror::new(
            &repo_path,
            "origin",
            "https://github.com/user/repo.git",
            config,
        );
        mirror.init_mirror()?;

        // Create and checkout a new branch
        mirror.checkout_branch("test-branch")?;

        // Verify we're on the new branch
        let output = Command::new("git")
            .args(["branch", "--show-current"])
            .current_dir(mirror.get_mirror_path())
            .output()?;

        let current_branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
        assert_eq!(current_branch, "test-branch");

        Ok(())
    }

    #[test]
    fn test_clean_removed_files() -> Result<()> {
        let temp = tempdir()?;
        let repo_path = temp.path().join(".dotman");
        fs::create_dir_all(&repo_path)?;

        let config = crate::config::Config::default();
        let mirror = GitMirror::new(
            &repo_path,
            "origin",
            "https://github.com/user/repo.git",
            config,
        );
        mirror.init_mirror()?;

        // Create multiple files and stage them
        let file1 = mirror.get_mirror_path().join("file1.txt");
        let file2 = mirror.get_mirror_path().join("file2.txt");
        let file3 = mirror.get_mirror_path().join("file3.txt");

        fs::write(&file1, "content1")?;
        fs::write(&file2, "content2")?;
        fs::write(&file3, "content3")?;

        // Stage all files so they're tracked by git
        Command::new("git")
            .args(["add", "."])
            .current_dir(mirror.get_mirror_path())
            .output()?;

        // Commit them so they're in the index
        mirror.commit("Initial files", "Test User")?;

        // Clean files not in the current list
        let current_files = vec![PathBuf::from("file1.txt"), PathBuf::from("file3.txt")];
        mirror.clean_removed_files(&current_files)?;

        // Verify file2 was removed
        assert!(file1.exists());
        assert!(!file2.exists());
        assert!(file3.exists());

        Ok(())
    }

    #[test]
    fn test_list_files() -> Result<()> {
        let temp = tempdir()?;
        let repo_path = temp.path().join(".dotman");
        fs::create_dir_all(&repo_path)?;

        let config = crate::config::Config::default();
        let mirror = GitMirror::new(
            &repo_path,
            "origin",
            "https://github.com/user/repo.git",
            config,
        );
        mirror.init_mirror()?;

        // Add files to git
        let file1 = mirror.get_mirror_path().join("file1.txt");
        let file2 = mirror.get_mirror_path().join("dir/file2.txt");

        fs::write(&file1, "content1")?;
        fs::create_dir_all(mirror.get_mirror_path().join("dir"))?;
        fs::write(&file2, "content2")?;

        // Add and commit files
        Command::new("git")
            .args(["add", "."])
            .current_dir(mirror.get_mirror_path())
            .output()?;

        Command::new("git")
            .args(["commit", "-m", "Initial commit"])
            .current_dir(mirror.get_mirror_path())
            .output()?;

        // List files
        let listed_files = mirror.list_files()?;
        assert_eq!(listed_files.len(), 2);
        assert!(listed_files.contains(&PathBuf::from("file1.txt")));
        assert!(listed_files.contains(&PathBuf::from("dir/file2.txt")));

        Ok(())
    }

    #[test]
    fn test_get_head_commit() -> Result<()> {
        let temp = tempdir()?;
        let repo_path = temp.path().join(".dotman");
        fs::create_dir_all(&repo_path)?;

        let config = crate::config::Config::default();
        let mirror = GitMirror::new(
            &repo_path,
            "origin",
            "https://github.com/user/repo.git",
            config,
        );
        mirror.init_mirror()?;

        // Create initial commit
        let test_file = mirror.get_mirror_path().join("test.txt");
        fs::write(&test_file, "content")?;

        let commit_id = mirror.commit("Test commit", "Test User")?;
        let head_commit = mirror.get_head_commit()?;

        assert_eq!(commit_id, head_commit);

        Ok(())
    }
}
