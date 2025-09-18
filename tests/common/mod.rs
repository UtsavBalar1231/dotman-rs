#![allow(dead_code)]

use anyhow::Result;
use dotman::DotmanContext;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// Test environment that provides isolated context for tests
pub struct TestEnvironment {
    /// Temporary directory for the test (automatically cleaned up)
    temp_dir: TempDir,
    /// Path to the home directory for this test
    pub home_dir: PathBuf,
    /// Path to the dotman repository
    pub repo_dir: PathBuf,
    /// Path to the config file
    pub config_path: PathBuf,
}

impl TestEnvironment {
    /// Create a new test environment with isolated directories
    pub fn new() -> Result<Self> {
        let temp_dir = TempDir::new()?;
        let home_dir = temp_dir.path().to_path_buf();
        let repo_dir = home_dir.join(".dotman");
        let config_path = home_dir.join(".config/dotman/config");

        Ok(Self {
            temp_dir,
            home_dir,
            repo_dir,
            config_path,
        })
    }

    /// Create a new test environment with a specific name prefix
    pub fn with_prefix(prefix: &str) -> Result<Self> {
        let temp_dir = TempDir::with_prefix(prefix)?;
        let home_dir = temp_dir.path().to_path_buf();
        let repo_dir = home_dir.join(".dotman");
        let config_path = home_dir.join(".config/dotman/config");

        Ok(Self {
            temp_dir,
            home_dir,
            repo_dir,
            config_path,
        })
    }

    /// Create a `DotmanContext` for this test environment
    pub fn create_context(&self) -> Result<DotmanContext> {
        // Create config directory if it doesn't exist
        if let Some(parent) = self.config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Create a context with explicit paths
        let context = DotmanContext::new_with_explicit_paths(
            self.repo_dir.clone(),
            self.config_path.clone(),
        )?;

        Ok(context)
    }

    /// Create a `DotmanContext` with pager disabled
    pub fn create_context_no_pager(&self) -> Result<DotmanContext> {
        // Create config directory if it doesn't exist
        if let Some(parent) = self.config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Create a context with explicit paths and no pager
        let context = DotmanContext::new_explicit(self.repo_dir.clone(), self.config_path.clone())?;

        Ok(context)
    }

    /// Initialize a dotman repository in this environment
    pub fn init_repo(&self) -> Result<DotmanContext> {
        let context = self.create_context_no_pager()?;
        context.ensure_repo_exists()?;

        // Create empty index
        let index = dotman::storage::index::Index::new();
        let index_path = context.repo_path.join(dotman::INDEX_FILE);
        index.save(&index_path)?;

        // Initialize reference manager
        let ref_manager = dotman::refs::RefManager::new(context.repo_path.clone());
        ref_manager.init()?;

        Ok(context)
    }

    /// Create a test file in the home directory
    pub fn create_test_file(&self, relative_path: &str, content: &str) -> Result<PathBuf> {
        let file_path = self.home_dir.join(relative_path);
        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&file_path, content)?;
        Ok(file_path)
    }

    /// Create a test file with specific permissions
    #[cfg(unix)]
    pub fn create_test_file_with_mode(
        &self,
        relative_path: &str,
        content: &str,
        mode: u32,
    ) -> Result<PathBuf> {
        use std::os::unix::fs::PermissionsExt;

        let file_path = self.create_test_file(relative_path, content)?;
        let permissions = std::fs::Permissions::from_mode(mode);
        std::fs::set_permissions(&file_path, permissions)?;
        Ok(file_path)
    }

    /// Create a symlink in the home directory
    #[cfg(unix)]
    pub fn create_symlink(&self, target: &str, link_name: &str) -> Result<PathBuf> {
        let link_path = self.home_dir.join(link_name);
        if let Some(parent) = link_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::os::unix::fs::symlink(target, &link_path)?;
        Ok(link_path)
    }

    /// Get the path to a file in the home directory
    pub fn file_path(&self, relative_path: &str) -> PathBuf {
        self.home_dir.join(relative_path)
    }

    /// Check if a file exists in the home directory
    pub fn file_exists(&self, relative_path: &str) -> bool {
        self.file_path(relative_path).exists()
    }

    /// Read a file from the home directory
    pub fn read_file(&self, relative_path: &str) -> Result<String> {
        Ok(std::fs::read_to_string(self.file_path(relative_path))?)
    }

    /// Get the home directory path as a string
    pub fn home_str(&self) -> String {
        self.home_dir.display().to_string()
    }

    /// Get the repo directory path as a string
    pub fn repo_str(&self) -> String {
        self.repo_dir.display().to_string()
    }

    /// Get the temporary directory path
    pub fn temp_path(&self) -> &Path {
        self.temp_dir.path()
    }
}

/// Helper to run a test with proper environment setup and cleanup
pub fn run_test_with_env<F>(test_fn: F) -> Result<()>
where
    F: FnOnce(&TestEnvironment) -> Result<()>,
{
    let env = TestEnvironment::new()?;
    test_fn(&env)
}

/// Helper to run a test that needs an initialized repository
pub fn run_test_with_repo<F>(test_fn: F) -> Result<()>
where
    F: FnOnce(&TestEnvironment, &DotmanContext) -> Result<()>,
{
    let env = TestEnvironment::new()?;
    let context = env.init_repo()?;
    test_fn(&env, &context)
}

/// Helper to modify file timestamps for testing
pub fn set_file_mtime(path: &Path, secs_ago: u64) -> Result<()> {
    use std::time::{Duration, SystemTime};

    let mtime = SystemTime::now() - Duration::from_secs(secs_ago);
    let atime = SystemTime::now();

    filetime::set_file_times(
        path,
        filetime::FileTime::from_system_time(atime),
        filetime::FileTime::from_system_time(mtime),
    )?;

    Ok(())
}

/// Helper to wait for filesystem timestamp resolution
/// This is more reliable than sleep for tests that need distinct timestamps
pub fn ensure_timestamp_change(path: &Path) -> Result<()> {
    let original_mtime = std::fs::metadata(path)?.modified()?;

    // Touch the file with a guaranteed different timestamp
    set_file_mtime(path, 1)?;

    let new_mtime = std::fs::metadata(path)?.modified()?;
    assert_ne!(original_mtime, new_mtime, "Timestamp should have changed");

    Ok(())
}

/// Assert that two paths point to the same content
pub fn assert_same_content(path1: &Path, path2: &Path) -> Result<()> {
    let content1 = std::fs::read(path1)?;
    let content2 = std::fs::read(path2)?;
    assert_eq!(content1, content2, "File contents should match");
    Ok(())
}

/// Assert that a file has specific permissions (Unix only)
#[cfg(unix)]
pub fn assert_permissions(path: &Path, expected_mode: u32) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let metadata = std::fs::metadata(path)?;
    let actual_mode = metadata.permissions().mode() & 0o777;
    assert_eq!(
        actual_mode, expected_mode,
        "Expected permissions {expected_mode:o}, got {actual_mode:o}"
    );
    Ok(())
}
