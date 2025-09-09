use crate::DotmanContext;
use crate::config::RemoteConfig;
use crate::refs::RefManager;
use anyhow::{Context, Result};

/// Trait providing common remote operations for command modules
pub trait RemoteOperations {
    /// Get a remote configuration by name with a helpful error message
    ///
    /// # Errors
    ///
    /// Returns an error if the remote does not exist
    fn get_remote_config(&self, name: &str) -> Result<&RemoteConfig>;

    /// Determine the remote and branch to use for push/pull operations
    ///
    /// If remote is None, uses the branch's upstream or defaults to "origin"
    /// If branch is None, uses the current branch
    ///
    /// # Errors
    ///
    /// Returns an error if the current branch cannot be determined or the remote does not exist
    fn determine_remote_and_branch(
        &self,
        remote: Option<&str>,
        branch: Option<&str>,
    ) -> Result<(String, String)>;

    /// Get the default remote name (usually "origin")
    fn get_default_remote(&self) -> &str;

    /// Check if a remote exists
    fn remote_exists(&self, name: &str) -> bool;

    /// Get all configured remotes
    fn list_remotes(&self) -> Vec<(&str, &RemoteConfig)>;
}

impl RemoteOperations for DotmanContext {
    fn get_remote_config(&self, name: &str) -> Result<&RemoteConfig> {
        self.config.get_remote(name).with_context(|| {
            format!("Remote '{name}' does not exist. Use 'dot remote add' to add it.")
        })
    }

    fn determine_remote_and_branch(
        &self,
        remote: Option<&str>,
        branch: Option<&str>,
    ) -> Result<(String, String)> {
        // Determine the branch
        let branch_name = if let Some(b) = branch {
            b.to_string()
        } else {
            // Use current branch
            let ref_manager = RefManager::new(self.repo_path.clone());
            ref_manager
                .current_branch()?
                .context("Not currently on any branch (detached HEAD)")?
        };

        // Determine the remote
        let remote_name = remote.map_or_else(
            || {
                self.config.branches.tracking.get(&branch_name).map_or_else(
                    || self.get_default_remote().to_string(),
                    |tracking| tracking.remote.clone(),
                )
            },
            std::string::ToString::to_string,
        );

        // Verify the remote exists
        if !self.remote_exists(&remote_name) {
            return Err(anyhow::anyhow!(
                "Remote '{}' does not exist. Use 'dot remote add' to add it.",
                remote_name
            ));
        }

        Ok((remote_name, branch_name))
    }

    fn get_default_remote(&self) -> &'static str {
        "origin"
    }

    fn remote_exists(&self, name: &str) -> bool {
        self.config.get_remote(name).is_some()
    }

    fn list_remotes(&self) -> Vec<(&str, &RemoteConfig)> {
        self.config
            .remotes
            .iter()
            .map(|(name, config)| (name.as_str(), config))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{BranchTracking, Config, RemoteType};
    use tempfile::TempDir;

    #[test]
    fn test_get_remote_config() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = Config::default();

        // Add a test remote
        config.set_remote(
            "origin".to_string(),
            RemoteConfig {
                remote_type: RemoteType::Git,
                url: Some("https://github.com/user/repo.git".to_string()),
            },
        );

        let ctx = DotmanContext {
            repo_path: temp_dir.path().to_path_buf(),
            config_path: temp_dir.path().join("config"),
            config,
            no_pager: false,
        };

        // Test existing remote
        let remote = ctx.get_remote_config("origin").unwrap();
        assert_eq!(remote.remote_type, RemoteType::Git);

        // Test non-existent remote
        let result = ctx.get_remote_config("nonexistent");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("does not exist"));
    }

    #[test]
    fn test_determine_remote_and_branch() {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path().to_path_buf();

        // Set up repo structure
        std::fs::create_dir_all(repo_path.join("refs/heads")).unwrap();
        std::fs::write(repo_path.join("HEAD"), "ref: refs/heads/main").unwrap();
        std::fs::write(repo_path.join("refs/heads/main"), "abc123").unwrap();

        let mut config = Config::default();

        // Add remotes
        config.set_remote(
            "origin".to_string(),
            RemoteConfig {
                remote_type: RemoteType::Git,
                url: Some("https://github.com/user/repo.git".to_string()),
            },
        );
        config.set_remote(
            "upstream".to_string(),
            RemoteConfig {
                remote_type: RemoteType::Git,
                url: Some("https://github.com/other/repo.git".to_string()),
            },
        );

        // Add tracking branch
        config.branches.tracking.insert(
            "main".to_string(),
            BranchTracking {
                remote: "upstream".to_string(),
                branch: "main".to_string(),
            },
        );

        let ctx = DotmanContext {
            repo_path,
            config_path: temp_dir.path().join("config"),
            config,
            no_pager: false,
        };

        // Test with explicit remote and branch
        let (remote, branch) = ctx
            .determine_remote_and_branch(Some("origin"), Some("feature"))
            .unwrap();
        assert_eq!(remote, "origin");
        assert_eq!(branch, "feature");

        // Test with current branch and tracking remote
        let (remote, branch) = ctx.determine_remote_and_branch(None, None).unwrap();
        assert_eq!(remote, "upstream"); // Should use tracking remote
        assert_eq!(branch, "main");

        // Test with explicit remote, current branch
        let (remote, branch) = ctx
            .determine_remote_and_branch(Some("origin"), None)
            .unwrap();
        assert_eq!(remote, "origin");
        assert_eq!(branch, "main");
    }

    #[test]
    fn test_remote_exists() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = Config::default();

        config.set_remote(
            "origin".to_string(),
            RemoteConfig {
                remote_type: RemoteType::Git,
                url: Some("https://github.com/user/repo.git".to_string()),
            },
        );

        let ctx = DotmanContext {
            repo_path: temp_dir.path().to_path_buf(),
            config_path: temp_dir.path().join("config"),
            config,
            no_pager: false,
        };

        assert!(ctx.remote_exists("origin"));
        assert!(!ctx.remote_exists("nonexistent"));
    }

    #[test]
    fn test_list_remotes() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = Config::default();

        config.set_remote(
            "origin".to_string(),
            RemoteConfig {
                remote_type: RemoteType::Git,
                url: Some("https://github.com/user/repo.git".to_string()),
            },
        );
        config.set_remote(
            "upstream".to_string(),
            RemoteConfig {
                remote_type: RemoteType::Git,
                url: Some("https://github.com/other/repo.git".to_string()),
            },
        );

        let ctx = DotmanContext {
            repo_path: temp_dir.path().to_path_buf(),
            config_path: temp_dir.path().join("config"),
            config,
            no_pager: false,
        };

        let remotes = ctx.list_remotes();
        assert_eq!(remotes.len(), 2);

        let names: Vec<&str> = remotes.iter().map(|(name, _)| *name).collect();
        assert!(names.contains(&"origin"));
        assert!(names.contains(&"upstream"));
    }
}
