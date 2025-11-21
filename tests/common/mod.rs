use anyhow::Result;
use dotman::DotmanContext;
use tempfile::TempDir;

/// Test repository fixture for consistent test setup
pub struct TestRepo {
    pub temp_dir: TempDir,
    pub ctx: DotmanContext,
}

impl TestRepo {
    /// Create a new test repository with initialized dotman structure
    pub fn new() -> Result<Self> {
        let temp_dir = TempDir::new()?;
        let repo_path = temp_dir.path().join(".dotman");
        let config_path = temp_dir.path().join(".config/dotman/config");

        let ctx = DotmanContext::new_explicit(repo_path.clone(), config_path)?;
        ctx.ensure_repo_exists()?;

        // Initialize index
        let index = dotman::storage::index::Index::new();
        let index_path = ctx.repo_path.join("index.bin");
        index.save(&index_path)?;

        // Initialize refs
        let ref_manager = dotman::refs::RefManager::new(ctx.repo_path.clone());
        ref_manager.init()?;

        Ok(Self { temp_dir, ctx })
    }

    /// Get the temporary directory path
    pub fn path(&self) -> &std::path::Path {
        self.temp_dir.path()
    }
}

impl Default for TestRepo {
    fn default() -> Self {
        Self::new().expect("Failed to create test repository")
    }
}
