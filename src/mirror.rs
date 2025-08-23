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
}

impl GitMirror {
    /// Create a new GitMirror instance
    pub fn new(repo_path: &Path, remote_name: &str, remote_url: &str) -> Self {
        let mirror_path = repo_path.join("mirrors").join(remote_name);
        Self {
            mirror_path,
            remote_name: remote_name.to_string(),
            remote_url: remote_url.to_string(),
        }
    }

    /// Initialize the mirror repository if it doesn't exist
    pub fn init_mirror(&self) -> Result<()> {
        if !self.mirror_path.exists() {
            fs::create_dir_all(&self.mirror_path).context("Failed to create mirror directory")?;

            // Initialize git repository
            let output = Command::new("git")
                .args(["init"])
                .current_dir(&self.mirror_path)
                .output()
                .context("Failed to initialize git repository")?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                anyhow::bail!("Git init failed: {}", stderr);
            }

            // Configure git user for the repository (required for commits)
            Command::new("git")
                .args(["config", "user.email", "dotman@localhost"])
                .current_dir(&self.mirror_path)
                .output()
                .context("Failed to configure git email")?;

            Command::new("git")
                .args(["config", "user.name", "Dotman"])
                .current_dir(&self.mirror_path)
                .output()
                .context("Failed to configure git name")?;

            // Add remote
            self.add_remote()?;
        } else {
            // Ensure remote is configured correctly
            self.update_remote()?;
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
                anyhow::bail!("Git remote add failed: {}", stderr);
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
            anyhow::bail!("Git remote set-url failed: {}", stderr);
        }

        Ok(())
    }

    /// Sync files from dotman storage to the mirror repository
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
    pub fn commit(&self, message: &str, author: &str) -> Result<String> {
        // Add all changes
        let output = Command::new("git")
            .args(["add", "-A"])
            .current_dir(&self.mirror_path)
            .output()
            .context("Failed to add files to git")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Git add failed: {}", stderr);
        }

        // Check if there are changes to commit
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
            anyhow::bail!("Git commit failed: {}", stderr);
        }

        // Get the commit ID
        self.get_head_commit()
    }

    /// Push changes to the remote repository
    pub fn push(&self, branch: &str) -> Result<()> {
        // First try to fetch to see if remote exists
        let _ = Command::new("git")
            .args(["fetch", "origin"])
            .current_dir(&self.mirror_path)
            .output();

        // Push to remote
        let output = Command::new("git")
            .args(["push", "origin", branch])
            .current_dir(&self.mirror_path)
            .output()
            .context("Failed to push to remote")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);

            // Try with --set-upstream if branch doesn't exist on remote
            if stderr.contains("has no upstream branch") || stderr.contains("src refspec") {
                let output = Command::new("git")
                    .args(["push", "--set-upstream", "origin", branch])
                    .current_dir(&self.mirror_path)
                    .output()
                    .context("Failed to push with --set-upstream")?;

                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    anyhow::bail!("Git push failed: {}", stderr);
                }
            } else {
                anyhow::bail!("Git push failed: {}", stderr);
            }
        }

        Ok(())
    }

    /// Pull changes from the remote repository
    pub fn pull(&self, branch: &str) -> Result<()> {
        // Fetch from remote
        let output = Command::new("git")
            .args(["fetch", "origin"])
            .current_dir(&self.mirror_path)
            .output()
            .context("Failed to fetch from remote")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Git fetch failed: {}", stderr);
        }

        // Check if branch exists locally
        let output = Command::new("git")
            .args(["rev-parse", "--verify", branch])
            .current_dir(&self.mirror_path)
            .output()?;

        if !output.status.success() {
            // Branch doesn't exist locally, create it from remote
            let output = Command::new("git")
                .args(["checkout", "-b", branch, &format!("origin/{}", branch)])
                .current_dir(&self.mirror_path)
                .output()
                .context("Failed to create local branch")?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                anyhow::bail!("Git checkout failed: {}", stderr);
            }
        } else {
            // Branch exists, checkout and pull
            let output = Command::new("git")
                .args(["checkout", branch])
                .current_dir(&self.mirror_path)
                .output()
                .context("Failed to checkout branch")?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                anyhow::bail!("Git checkout failed: {}", stderr);
            }

            // Pull changes
            let output = Command::new("git")
                .args(["pull", "origin", branch])
                .current_dir(&self.mirror_path)
                .output()
                .context("Failed to pull from remote")?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                anyhow::bail!("Git pull failed: {}", stderr);
            }
        }

        Ok(())
    }

    /// Get the current HEAD commit ID
    pub fn get_head_commit(&self) -> Result<String> {
        let output = Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(&self.mirror_path)
            .output()
            .context("Failed to get HEAD commit")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Git rev-parse failed: {}", stderr);
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    /// Get the path to the mirror repository
    pub fn get_mirror_path(&self) -> &Path {
        &self.mirror_path
    }

    /// List all files in the mirror repository
    pub fn list_files(&self) -> Result<Vec<PathBuf>> {
        let output = Command::new("git")
            .args(["ls-files"])
            .current_dir(&self.mirror_path)
            .output()
            .context("Failed to list files in mirror")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Git ls-files failed: {}", stderr);
        }

        let files = String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(PathBuf::from)
            .collect();

        Ok(files)
    }

    /// Checkout a specific branch in the mirror
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
                    anyhow::bail!("Git checkout -b failed: {}", stderr);
                }
            } else {
                anyhow::bail!("Git checkout failed: {}", stderr);
            }
        }

        Ok(())
    }

    /// Remove files from the mirror that are not in the provided list
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

        let mirror = GitMirror::new(&repo_path, "origin", "https://github.com/user/repo.git");
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

        let mirror = GitMirror::new(&repo_path, "origin", "https://github.com/user/repo.git");
        mirror.init_mirror()?;

        let files = vec![(source_file.clone(), PathBuf::from("dest.txt"))];
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

        let mirror = GitMirror::new(&repo_path, "origin", "https://github.com/user/repo.git");
        mirror.init_mirror()?;

        // Add a file to commit
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

        let mirror = GitMirror::new(&repo_path, "origin", "https://github.com/user/repo.git");
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

        let mirror = GitMirror::new(&repo_path, "origin", "https://github.com/user/repo.git");
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

        let mirror = GitMirror::new(&repo_path, "origin", "https://github.com/user/repo.git");
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
        let files = mirror.list_files()?;
        assert_eq!(files.len(), 2);
        assert!(files.contains(&PathBuf::from("file1.txt")));
        assert!(files.contains(&PathBuf::from("dir/file2.txt")));

        Ok(())
    }

    #[test]
    fn test_get_head_commit() -> Result<()> {
        let temp = tempdir()?;
        let repo_path = temp.path().join(".dotman");
        fs::create_dir_all(&repo_path)?;

        let mirror = GitMirror::new(&repo_path, "origin", "https://github.com/user/repo.git");
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
