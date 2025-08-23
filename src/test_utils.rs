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
                config: config.clone(),
            };

            Ok(Self {
                temp_dir,
                repo_path,
                config_path,
                context,
            })
        }

        pub fn create_file(&self, name: &str, content: &str) -> Result<PathBuf> {
            let path = self.temp_dir.path().join(name);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&path, content)?;
            Ok(path)
        }

        pub fn create_commit(&self, id: &str, message: &str) -> Result<()> {
            let commit = crate::storage::Commit {
                id: id.to_string(),
                parent: None,
                message: message.to_string(),
                author: "Test User".to_string(),
                timestamp: chrono::Utc::now().timestamp(),
                tree_hash: "test_tree".to_string(),
            };

            let snapshot = crate::storage::snapshots::Snapshot {
                commit,
                files: Default::default(),
            };

            // Serialize and compress snapshot
            let serialized = crate::utils::serialization::serialize(&snapshot)?;
            let compressed = zstd::stream::encode_all(&serialized[..], 3)?;

            let snapshot_path = self.repo_path.join("commits").join(format!("{}.zst", id));
            fs::write(&snapshot_path, compressed)?;

            // Update HEAD
            fs::write(self.repo_path.join("HEAD"), id)?;
            Ok(())
        }

        pub fn set_config_remote(
            &mut self,
            remote_type: crate::config::RemoteType,
            url: Option<String>,
        ) -> Result<()> {
            // Add a remote named "origin" with the specified type and url
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

        // Create default config
        let mut config = Config::default();
        config.core.repo_path = repo_path.clone();
        config.save(&config_path)?;

        let ctx = DotmanContext {
            repo_path,
            config_path,
            config,
        };

        Ok((temp, ctx))
    }
}
