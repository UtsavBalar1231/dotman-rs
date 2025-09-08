#[cfg(test)]
pub mod fixtures {
    use crate::{DotmanContext, config::Config};
    use anyhow::Result;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    pub struct TestRepository {
        pub temp_dir: TempDir,
        pub repo_path: PathBuf,
        pub config_path: PathBuf,
        pub context: DotmanContext,
    }

    impl TestRepository {
        /// Create a new test repository
        ///
        /// # Errors
        ///
        /// Returns an error if failed to create repository structure
        pub fn new() -> Result<Self> {
            let temp_dir = tempfile::tempdir()?;
            let repo_path = temp_dir.path().join(".dotman");
            let config_path = temp_dir.path().join("config.toml");

            // Create repository structure
            fs::create_dir_all(&repo_path)?;
            fs::create_dir_all(repo_path.join("commits"))?;
            fs::create_dir_all(repo_path.join("objects"))?;

            // Create initial index
            let index = crate::storage::index::Index::new();
            index.save(&repo_path.join("index.bin"))?;

            // Create config
            let mut config = Config::default();
            config.core.repo_path = repo_path.clone();
            config.save(&config_path)?;

            let context = DotmanContext {
                repo_path: repo_path.clone(),
                config_path: config_path.clone(),
                config,
                no_pager: true,
            };

            Ok(Self {
                temp_dir,
                repo_path,
                config_path,
                context,
            })
        }

        /// Create a test file in the repository
        ///
        /// # Errors
        ///
        /// Returns an error if failed to create file
        pub fn create_file(&self, name: &str, content: &str) -> Result<PathBuf> {
            let path = self.temp_dir.path().join(name);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&path, content)?;
            Ok(path)
        }

        /// Create a test commit
        ///
        /// # Errors
        ///
        /// Returns an error if failed to create or save commit
        pub fn create_commit(&self, id: &str, message: &str) -> Result<()> {
            let commit_id = test_commit_id(id);
            let commit = crate::storage::Commit {
                id: commit_id.clone(),
                parent: None,
                message: message.to_string(),
                author: "Test User".to_string(),
                timestamp: chrono::Utc::now().timestamp(),
                tree_hash: "test_tree".to_string(),
            };

            let snapshot = crate::storage::snapshots::Snapshot {
                commit,
                files: std::collections::HashMap::default(),
            };

            // Serialize and compress snapshot
            let serialized = crate::utils::serialization::serialize(&snapshot)?;
            let compressed = zstd::stream::encode_all(&serialized[..], 3)?;

            let snapshot_path = self
                .repo_path
                .join("commits")
                .join(format!("{}.zst", &commit_id));
            fs::write(&snapshot_path, compressed)?;

            // Update HEAD
            fs::write(self.repo_path.join("HEAD"), &commit_id)?;
            Ok(())
        }

        /// Set remote configuration for testing
        ///
        /// # Errors
        ///
        /// Returns an error if failed to save configuration
        pub fn set_config_remote(
            &mut self,
            remote_type: crate::config::RemoteType,
            url: Option<String>,
        ) -> Result<()> {
            let remote_config = crate::config::RemoteConfig { remote_type, url };
            self.context
                .config
                .remotes
                .insert("origin".to_string(), remote_config);
            self.context.config.save(&self.config_path)?;
            Ok(())
        }
    }

    // Helper function to create a basic test context
    /// Create a basic test context
    ///
    /// # Errors
    ///
    /// Returns an error if failed to create test environment
    pub fn create_test_context() -> Result<(TempDir, DotmanContext)> {
        let temp = tempfile::tempdir()?;
        let repo_path = temp.path().join(".dotman");
        let config_path = temp.path().join("config.toml");

        // Create repo structure
        fs::create_dir_all(&repo_path)?;
        fs::create_dir_all(repo_path.join("commits"))?;
        fs::create_dir_all(repo_path.join("objects"))?;

        // Create empty index
        let index = crate::storage::index::Index::new();
        index.save(&repo_path.join("index.bin"))?;

        let ref_manager = crate::refs::RefManager::new(repo_path.clone());
        ref_manager.init()?;

        // Create default config
        let mut config = Config::default();
        config.core.repo_path = repo_path.clone();
        config.save(&config_path)?;

        let ctx = DotmanContext {
            repo_path,
            config_path,
            config,
            no_pager: true,
        };

        Ok((temp, ctx))
    }

    /// Generates a valid 32-character hexadecimal commit ID for tests
    /// Pads the input with zeros to ensure it's exactly 32 characters
    #[must_use]
    pub fn test_commit_id(suffix: &str) -> String {
        if suffix.len() >= 32 {
            // If suffix is already 32+ chars, take first 32 and ensure they're all hex
            suffix[..32]
                .chars()
                .map(|c| if c.is_ascii_hexdigit() { c } else { '0' })
                .collect()
        } else {
            // Pad with zeros at the start to make exactly 32 characters
            format!("{suffix:0>32}")
        }
    }

    /// Generates a sequence of test commit IDs (01, 02, 03, etc.)
    #[must_use]
    pub fn test_commit_ids(count: usize) -> Vec<String> {
        (1..=count)
            .map(|i| test_commit_id(&format!("{i:02}")))
            .collect()
    }
}
