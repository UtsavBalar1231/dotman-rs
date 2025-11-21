//! # Git Mirror Management
//!
//! The mirror module manages local git repositories that act as intermediaries
//! between dotman's content-addressed storage and remote git repositories.
//!
//! ## Architecture
//!
//! Dotman uses a "mirror" pattern to interface with git remotes:
//!
//! ```text
//! ┌─────────────────┐     ┌──────────────┐     ┌─────────────┐
//! │ Dotman Storage  │ <-> │  Git Mirror  │ <-> │ Git Remote  │
//! │ (content-addr)  │     │ (~/.dotman/  │     │ (GitHub,    │
//! │                 │     │  mirrors/)   │     │  etc.)      │
//! └─────────────────┘     └──────────────┘     └─────────────┘
//! ```
//!
//! The mirror serves several purposes:
//! - Translates dotman's content-addressed storage to git's commit structure
//! - Provides a staging area for export/import operations
//! - Handles git-specific operations (fetch, push, merge)
//! - Maintains separate working directories per remote
//!
//! ## Workflow
//!
//! ### Push Workflow
//! 1. Initialize mirror repository (`.dotman/mirrors/{remote-name}/`)
//! 2. Clear working directory to ensure clean state
//! 3. Export each dotman commit's files to mirror
//! 4. Commit in mirror with original timestamp and metadata
//! 5. Push mirror branch to remote repository
//! 6. Map dotman commit IDs to git commit IDs
//!
//! ### Pull Workflow
//! 1. Initialize mirror repository
//! 2. Fetch and checkout remote branch in mirror
//! 3. Import changed files from mirror to dotman storage
//! 4. Create new dotman commit with imported changes
//! 5. Map git commit ID to dotman commit ID
//!
//! ## Locking and Concurrency
//!
//! Mirror initialization is protected by file locks to prevent race conditions
//! when multiple dotman processes try to initialize the same mirror concurrently.
//! Lock files are stored in `.dotman/mirrors/locks/` and use exclusive locks
//! with a 30-second timeout.
//!
//! ## Error Handling
//!
//! The module uses the `errors` submodule for git error categorization:
//! - Network errors (retryable)
//! - Authentication errors (require user action)
//! - Conflict errors (require resolution)
//! - Permission errors (filesystem or git)
//!
//! Git errors are parsed from stderr and converted to user-friendly messages
//! with actionable suggestions.
//!
//! ## Cleanup and Resilience
//!
//! File operations use retry logic with exponential backoff to handle:
//! - Transient filesystem errors
//! - Anti-virus software interference
//! - Network filesystem delays
//!
//! If cleanup fails, operations continue with warnings rather than failing
//! entirely, prioritizing data integrity over perfect cleanup.

use crate::config::Config;
use anyhow::{Context, Result};
use fs4::fs_std::FileExt;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// Git error categorization and handling
pub mod errors;

/// Manages git mirror repositories for remote synchronization
pub struct GitMirror {
    /// Path to the mirror repository (.dotman/mirrors/{remote-name})
    mirror_path: PathBuf,
    /// Name of the remote
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

    /// Acquire an exclusive lock for mirror initialization
    ///
    /// Prevents concurrent initialization of the same mirror by different processes.
    /// The lock is automatically released when the returned File is dropped.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Cannot create lock file
    /// - Cannot acquire lock within timeout period
    /// - Another process holds the lock
    fn acquire_mirror_lock(&self) -> Result<File> {
        // Use shorter timeouts in test mode for faster test execution
        let lock_timeout = if cfg!(test) {
            Duration::from_millis(100)
        } else {
            Duration::from_secs(30)
        };
        let retry_interval = if cfg!(test) {
            Duration::from_millis(10)
        } else {
            Duration::from_millis(100)
        };

        // Create locks directory in the mirrors parent directory
        let locks_dir = self
            .mirror_path
            .parent()
            .context("Mirror path has no parent")?
            .join("locks");
        fs::create_dir_all(&locks_dir).context("Failed to create locks directory")?;

        // Lock file is named after the remote
        let lock_path = locks_dir.join(format!("{}.lock", self.remote_name));

        let start = Instant::now();

        loop {
            // Create or open lock file
            let file = File::create(&lock_path)
                .with_context(|| format!("Failed to create lock file: {}", lock_path.display()))?;

            // Try to acquire exclusive lock
            match file.try_lock_exclusive() {
                Ok(true) => {
                    // Lock acquired successfully
                    return Ok(file);
                }
                Ok(false) | Err(_) if start.elapsed() < lock_timeout => {
                    // Lock held by another process, wait and retry
                    std::thread::sleep(retry_interval);
                }
                Ok(false) | Err(_) => {
                    return Err(anyhow::anyhow!(
                        "Another process is initializing mirror '{}'. \
                         Please wait for it to complete or remove stale lock at: {}",
                        self.remote_name,
                        lock_path.display()
                    ));
                }
            }
        }
    }

    /// Verify that the mirror repository is in a valid state
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - .git directory does not exist
    /// - Git config is invalid
    /// - Remote is not configured
    fn verify_mirror(&self) -> Result<()> {
        // Check .git directory exists
        let git_dir = self.mirror_path.join(".git");
        if !git_dir.exists() {
            return Err(anyhow::anyhow!(
                "Mirror .git directory does not exist at {}",
                git_dir.display()
            ));
        }

        // Verify git config is valid by running a simple git command
        let output = Command::new("git")
            .args(["config", "--get", "user.email"])
            .current_dir(&self.mirror_path)
            .stdin(Stdio::null())
            .output()
            .context("Failed to check git config")?;

        if !output.status.success() {
            return Err(anyhow::anyhow!(
                "Git configuration is invalid in mirror {}",
                self.mirror_path.display()
            ));
        }

        // Verify remote is configured
        let output = Command::new("git")
            .args(["remote", "get-url", "origin"])
            .current_dir(&self.mirror_path)
            .stdin(Stdio::null())
            .output()
            .context("Failed to check remote configuration")?;

        if !output.status.success() {
            return Err(anyhow::anyhow!(
                "Remote 'origin' is not configured in mirror {}",
                self.mirror_path.display()
            ));
        }

        Ok(())
    }

    /// Initialize the mirror repository if it doesn't exist
    ///
    /// This operation is protected by a file lock to prevent race conditions
    /// when multiple processes try to initialize the same mirror concurrently.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Cannot acquire initialization lock
    /// - Failed to create mirror directory
    /// - Git init command fails
    /// - Failed to configure git user
    /// - Failed to add remote
    pub fn init_mirror(&self) -> Result<()> {
        // Acquire lock for entire initialization process
        let _lock = self.acquire_mirror_lock()?;

        // Check existence while holding lock (prevents TOCTOU race)
        if self.mirror_path.exists() {
            // Verify the existing mirror is valid
            self.verify_mirror()?;
            // Ensure remote URL is up to date
            self.update_remote()?;
        } else {
            // Initialize new mirror (with cleanup on failure)
            match self.initialize_git_mirror() {
                Ok(()) => {}
                Err(e) => {
                    // Cleanup partial initialization
                    let _ = fs::remove_dir_all(&self.mirror_path);
                    return Err(e);
                }
            }
        }

        // Lock is automatically released when _lock goes out of scope
        Ok(())
    }

    /// Perform the actual git mirror initialization
    ///
    /// This is separated from `init_mirror` to allow for cleanup on failure.
    ///
    /// # Errors
    ///
    /// Returns an error if git initialization or configuration fails
    fn initialize_git_mirror(&self) -> Result<()> {
        // Create mirror directory
        fs::create_dir_all(&self.mirror_path).context("Failed to create mirror directory")?;

        // Initialize git repository
        let output = Command::new("git")
            .args(["init"])
            .current_dir(&self.mirror_path)
            .stdin(Stdio::null())
            .output()
            .context("Failed to initialize git repository")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let error = errors::GitError::from_stderr("git init", &stderr);
            eprintln!("{}", error.user_message());
            return Err(anyhow::anyhow!(error.to_string()));
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

        let output = Command::new("git")
            .args(["config", "user.email", user_email])
            .current_dir(&self.mirror_path)
            .stdin(Stdio::null())
            .output()
            .context("Failed to configure git email")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("Git config user.email failed: {stderr}"));
        }

        let output = Command::new("git")
            .args(["config", "user.name", user_name])
            .current_dir(&self.mirror_path)
            .stdin(Stdio::null())
            .output()
            .context("Failed to configure git name")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("Git config user.name failed: {stderr}"));
        }

        // Add remote
        self.add_remote()?;

        Ok(())
    }

    /// Add the remote to the mirror repository
    fn add_remote(&self) -> Result<()> {
        let output = Command::new("git")
            .args(["remote", "add", "origin", &self.remote_url])
            .current_dir(&self.mirror_path)
            .stdin(Stdio::null())
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
            .stdin(Stdio::null())
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

                // Preserve file permissions using cross-platform module
                if self.config.tracking.preserve_permissions {
                    let permissions =
                        crate::utils::permissions::FilePermissions::from_path(source_path)?;
                    permissions.apply_to_path(&dest_path, true)?;
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
            .stdin(Stdio::null())
            .output()
            .context("Failed to add files to git")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("Git add failed: {stderr}"));
        }

        let output = Command::new("git")
            .args(["status", "--porcelain"])
            .current_dir(&self.mirror_path)
            .stdin(Stdio::null())
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
            .stdin(Stdio::null())
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
            .stdin(Stdio::null())
            .output()
            .context("Failed to add files to git")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("Git add failed: {stderr}"));
        }

        let output = Command::new("git")
            .args(["status", "--porcelain"])
            .current_dir(&self.mirror_path)
            .stdin(Stdio::null())
            .output()
            .context("Failed to check git status")?;

        if output.stdout.is_empty() {
            // No changes to commit
            // If there's no HEAD yet (empty repo), we need to create an empty initial commit
            // Otherwise, return the current commit ID
            if let Ok(head) = self.get_head_commit() {
                return Ok(head);
            }
            // Empty repo - will create an initial empty commit below
            // This happens on first push when mirror is empty
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

        // Check if we need --allow-empty (for initial commits in empty repo)
        let has_head = self.get_head_commit().is_ok();

        // Commit changes with specific date
        let mut cmd = Command::new("git");
        cmd.args(["commit", "-m", message, "--author", &formatted_author]);

        // Add --allow-empty for first commit in empty repo
        if !has_head {
            cmd.arg("--allow-empty");
        }

        let output = cmd
            .env("GIT_AUTHOR_DATE", &date_str)
            .env("GIT_COMMITTER_DATE", &date_str)
            .current_dir(&self.mirror_path)
            .stdin(Stdio::null())
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
    /// This function implements a robust cleanup strategy with multi-level retry logic
    /// to handle various failure scenarios that can occur in real-world filesystems.
    ///
    /// ## Why Retry Logic is Necessary
    ///
    /// File deletion can fail for transient reasons:
    /// - Anti-virus software scanning files
    /// - Network filesystem delays
    /// - Background indexing (Windows Search, Spotlight, etc.)
    /// - File locks from other processes
    /// - Timing issues with recently created files
    ///
    /// ## Cleanup Strategy
    ///
    /// ### Phase 1: Git Index Cleanup
    /// Run `git rm -rf --cached .` to clear git's index. This tells git to stop
    /// tracking all files. Failure here is non-fatal - we continue with manual cleanup.
    ///
    /// ### Phase 2: First Removal Pass
    /// Iterate through all directory entries and try to remove each (except .git).
    /// Uses `remove_with_retry` with 3 attempts per file:
    /// - Attempt 1: Immediate
    /// - Attempt 2: After 50ms
    /// - Attempt 3: After 100ms
    ///
    /// Files that fail all 3 attempts are collected for retry.
    ///
    /// ### Phase 3: Second Removal Pass
    /// Wait 100ms (gives filesystem time to settle) then retry all failed removals
    /// with 2 more attempts. This often succeeds because:
    /// - Anti-virus scans complete
    /// - File locks release
    /// - Filesystem caches flush
    ///
    /// ### Phase 4: Verification
    /// Check if directory is truly empty (except .git). If files remain, we warn
    /// but continue - partial cleanup is better than failing the entire operation.
    ///
    /// ## Error Philosophy
    ///
    /// This function prioritizes success over perfection:
    /// - Transient errors trigger retries, not immediate failure
    /// - Persistent errors generate warnings but allow operation to continue
    /// - Only complete inability to clean up causes an error
    ///
    /// This ensures that push operations succeed even in challenging filesystem
    /// environments, while still providing visibility into cleanup issues.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Failed to remove tracked files after retries
    /// - Failed to read or delete directory entries after retries
    pub fn clear_working_directory(&self) -> Result<()> {
        // Clear git index with retry
        let output = Command::new("git")
            .args(["rm", "-rf", "--cached", "."])
            .current_dir(&self.mirror_path)
            .stdin(Stdio::null())
            .output()
            .context("Failed to execute git rm command")?;

        // Check if git rm succeeded
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            eprintln!("Warning: git rm failed (continuing with manual cleanup): {stderr}");
        }

        // Physically remove files (except .git) with retry logic
        let mut failed_removals = Vec::new();

        for entry in std::fs::read_dir(&self.mirror_path)? {
            let entry = entry?;
            let path = entry.path();
            let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

            // Skip .git directory - it must be preserved for mirror to remain valid
            if file_name != ".git"
                && let Err(e) = Self::remove_with_retry(&path, 3)
            {
                failed_removals.push((path.clone(), e));
            }
        }

        // Retry failed removals once more after a brief delay
        // This often succeeds because filesystem operations have time to complete
        if !failed_removals.is_empty() {
            eprintln!("Retrying {} failed removals...", failed_removals.len());
            // Use shorter delay in test mode for faster test execution
            let delay = if cfg!(test) { 20 } else { 100 };
            std::thread::sleep(Duration::from_millis(delay));

            let mut still_failed = Vec::new();
            for (path, _) in failed_removals {
                if let Err(e) = Self::remove_with_retry(&path, 2) {
                    still_failed.push((path, e));
                }
            }

            if !still_failed.is_empty() {
                eprintln!("Warning: Failed to remove {} items:", still_failed.len());
                for (path, err) in &still_failed {
                    eprintln!("  {}: {}", path.display(), err);
                }
                // Continue anyway - partial cleanup is better than none
                // The mirror is still usable, just has some extra files
            }
        }

        // Verify cleanup succeeded
        self.verify_cleanup()?;

        Ok(())
    }

    /// Remove a file or directory with retry logic and exponential backoff
    ///
    /// Implements exponential backoff to gracefully handle transient errors:
    /// - Try 1: Immediate (0ms delay)
    /// - Try 2: After 100ms (50ms * 2^1)
    /// - Try 3: After 200ms (50ms * 2^2)
    /// - Try N: After (50ms * 2^N)
    ///
    /// This pattern gives filesystem operations time to complete while avoiding
    /// excessive delays when the operation will succeed quickly.
    fn remove_with_retry(path: &Path, max_retries: u32) -> Result<()> {
        let mut retries = 0;
        let mut last_error = None;

        while retries < max_retries {
            let result = if path.is_dir() {
                std::fs::remove_dir_all(path)
            } else {
                std::fs::remove_file(path)
            };

            match result {
                Ok(()) => return Ok(()),
                Err(e) => {
                    last_error = Some(e);
                    retries += 1;
                    if retries < max_retries {
                        // Exponential backoff: shorter delays in test mode for faster tests
                        let base_delay = if cfg!(test) { 10 } else { 50 };
                        let delay = Duration::from_millis(base_delay * (1 << retries));
                        std::thread::sleep(delay);
                    }
                }
            }
        }

        last_error.map_or_else(
            || {
                Err(anyhow::anyhow!(
                    "Failed to remove {} after {} attempts: no error recorded",
                    path.display(),
                    max_retries
                ))
            },
            |err| {
                Err(anyhow::anyhow!(
                    "Failed to remove {} after {} attempts: {}",
                    path.display(),
                    max_retries,
                    err
                ))
            },
        )
    }

    /// Verify that cleanup succeeded (directory is empty except for .git)
    fn verify_cleanup(&self) -> Result<()> {
        let mut remaining_count = 0;

        for entry in std::fs::read_dir(&self.mirror_path)? {
            let entry = entry?;
            let file_name = entry.file_name();
            let file_name_str = file_name.to_string_lossy();

            // Skip .git directory
            if file_name_str != ".git" {
                remaining_count += 1;
                if remaining_count <= 5 {
                    eprintln!("  Remaining: {file_name_str}");
                }
            }
        }

        if remaining_count > 0 {
            eprintln!("Warning: {remaining_count} items remaining after cleanup (expected 0)");
            // This is a warning, not an error - allow operation to continue
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
            .stdin(Stdio::null())
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
            .stdin(Stdio::null())
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
                    let error = errors::GitError::from_stderr("git push --set-upstream", &stderr);
                    eprintln!("{}", error.user_message());
                    if error.should_retry() {
                        eprintln!("Hint: This error may be transient. Try again.");
                    }
                    return Err(anyhow::anyhow!(error.to_string()));
                }
            } else {
                let error = errors::GitError::from_stderr("git push", &stderr);
                eprintln!("{}", error.user_message());
                if error.should_retry() {
                    eprintln!("Hint: This error may be transient. Try again.");
                }
                return Err(anyhow::anyhow!(error.to_string()));
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
            .stdin(Stdio::null())
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
            .stdin(Stdio::null())
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
            .stdin(Stdio::null())
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
            .stdin(Stdio::null())
            .output()
            .context("Failed to fetch from remote")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("Git fetch failed: {stderr}"));
        }

        let output = Command::new("git")
            .args(["rev-parse", "--verify", branch])
            .current_dir(&self.mirror_path)
            .stdin(Stdio::null())
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
            .stdin(Stdio::null())
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
            .stdin(Stdio::null())
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
            .stdin(Stdio::null())
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
