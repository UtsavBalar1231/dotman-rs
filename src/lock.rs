//! Operation locking to prevent concurrent remote operations on the same branch
//!
//! This module provides per-branch operation locking to prevent concurrent push/pull/fetch
//! operations from corrupting repository state. Locks are automatically released when dropped.

use anyhow::{Context, Result, bail};
use fs4::fs_std::FileExt;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

/// Types of remote operations that can be locked
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationType {
    /// Push operation
    Push,
    /// Pull operation
    Pull,
    /// Fetch operation
    Fetch,
}

impl OperationType {
    /// Get the string representation of the operation type
    const fn as_str(self) -> &'static str {
        match self {
            Self::Push => "push",
            Self::Pull => "pull",
            Self::Fetch => "fetch",
        }
    }
}

/// Holds an exclusive lock on a branch for a specific operation
///
/// The lock is automatically released when this struct is dropped.
pub struct OperationLock {
    /// Lock file handle
    lock_file: File,
    /// Path to the lock file (for error messages)
    lock_path: PathBuf,
}

impl OperationLock {
    /// Acquire an exclusive lock for an operation on a branch
    ///
    /// # Arguments
    ///
    /// * `repo_path` - Path to the dotman repository
    /// * `operation` - Type of operation being performed
    /// * `branch` - Name of the branch
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Cannot create locks directory
    /// - Cannot acquire lock within timeout period
    /// - Another operation is already in progress on this branch
    pub fn acquire(repo_path: &Path, operation: OperationType, branch: &str) -> Result<Self> {
        let locks_dir = repo_path.join("locks");
        fs::create_dir_all(&locks_dir).context("Failed to create locks directory")?;

        // Clean up stale locks before attempting to acquire
        Self::cleanup_stale_locks(&locks_dir)?;

        // Use branch name for lock file - only one operation per branch at a time
        let lock_path = locks_dir.join(format!("{branch}.lock"));

        // Try to acquire lock
        let lock_file = Self::try_acquire_lock(&lock_path, operation, branch)?;

        Ok(Self {
            lock_file,
            lock_path,
        })
    }

    /// Try to acquire the lock file
    fn try_acquire_lock(lock_path: &Path, operation: OperationType, branch: &str) -> Result<File> {
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

        let start = Instant::now();

        loop {
            // Create or open lock file
            let file = File::create(lock_path)
                .with_context(|| format!("Failed to create lock file: {}", lock_path.display()))?;

            // Try to acquire exclusive lock
            match file.try_lock_exclusive() {
                Ok(true) => {
                    // Lock acquired successfully
                    // Write operation info to lock file for debugging
                    use std::io::Write;
                    let mut file_ref = &file;
                    let _ = writeln!(
                        file_ref,
                        "operation={}\nbranch={}\npid={}\ntime={}",
                        operation.as_str(),
                        branch,
                        std::process::id(),
                        humantime::format_rfc3339(SystemTime::now())
                    );
                    return Ok(file);
                }
                Ok(false) | Err(_) if start.elapsed() < lock_timeout => {
                    // Lock held by another process, wait and retry
                    std::thread::sleep(retry_interval);
                }
                Ok(false) | Err(_) => {
                    bail!(
                        "Another {} operation is already in progress on branch '{}'. \
                         Please wait for it to complete or remove stale lock at: {}",
                        operation.as_str(),
                        branch,
                        lock_path.display()
                    );
                }
            }
        }
    }

    /// Clean up stale lock files (older than 5 minutes)
    ///
    /// This handles cases where a process crashed without releasing its lock.
    fn cleanup_stale_locks(locks_dir: &Path) -> Result<()> {
        const STALE_THRESHOLD: Duration = Duration::from_secs(300); // 5 minutes

        if !locks_dir.exists() {
            return Ok(());
        }

        let entries = fs::read_dir(locks_dir).context("Failed to read locks directory")?;

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_none_or(|ext| ext != "lock") {
                continue;
            }

            // Check file modification time
            if let Ok(metadata) = entry.metadata()
                && let Ok(modified) = metadata.modified()
                && let Ok(elapsed) = modified.elapsed()
                && elapsed > STALE_THRESHOLD
            {
                // Try to remove stale lock
                if let Err(e) = fs::remove_file(&path) {
                    eprintln!(
                        "Warning: Failed to remove stale lock {}: {}",
                        path.display(),
                        e
                    );
                }
            }
        }

        Ok(())
    }

    /// Release the lock explicitly (normally handled by Drop)
    ///
    /// # Errors
    ///
    /// Returns an error if the unlock operation fails, such as when the lock file
    /// cannot be unlocked due to I/O errors
    pub fn release(self) -> Result<()> {
        self.lock_file.unlock()?;
        // Remove lock file
        if let Err(e) = fs::remove_file(&self.lock_path) {
            eprintln!(
                "Warning: Failed to remove lock file {}: {}",
                self.lock_path.display(),
                e
            );
        }
        Ok(())
    }
}

impl Drop for OperationLock {
    fn drop(&mut self) {
        // Unlock file (happens automatically, but being explicit)
        let _ = self.lock_file.unlock();

        // Remove lock file
        if let Err(e) = fs::remove_file(&self.lock_path) {
            eprintln!(
                "Warning: Failed to remove lock file during cleanup {}: {}",
                self.lock_path.display(),
                e
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_acquire_and_release() {
        let temp = TempDir::new().unwrap();
        let lock = OperationLock::acquire(temp.path(), OperationType::Push, "main").unwrap();
        assert!(lock.lock_path.exists());
        lock.release().unwrap();
    }

    #[test]
    fn test_concurrent_locks_fail() {
        let temp = TempDir::new().unwrap();
        let _lock1 = OperationLock::acquire(temp.path(), OperationType::Push, "main").unwrap();

        // Second lock should fail quickly in test mode
        let start = Instant::now();
        let result = OperationLock::acquire(temp.path(), OperationType::Push, "main");
        let elapsed = start.elapsed();

        assert!(result.is_err(), "Second lock acquisition should fail");
        assert!(
            elapsed < Duration::from_millis(200),
            "Lock should fail quickly in test mode (took {elapsed:?})"
        );
    }

    #[test]
    fn test_different_operations_allowed() {
        let temp = TempDir::new().unwrap();
        let _lock1 = OperationLock::acquire(temp.path(), OperationType::Push, "main").unwrap();

        // Different operation on same branch should fail quickly in test mode
        let start = Instant::now();
        let result = OperationLock::acquire(temp.path(), OperationType::Pull, "main");
        let elapsed = start.elapsed();

        assert!(
            result.is_err(),
            "Different operation on same branch should fail"
        );
        assert!(
            elapsed < Duration::from_millis(200),
            "Lock should fail quickly in test mode (took {elapsed:?})"
        );
    }

    #[test]
    fn test_different_branches_allowed() {
        let temp = TempDir::new().unwrap();
        let _lock1 = OperationLock::acquire(temp.path(), OperationType::Push, "main").unwrap();

        // Same operation on different branch should succeed
        let lock2 = OperationLock::acquire(temp.path(), OperationType::Push, "feature");
        assert!(lock2.is_ok());
    }
}
