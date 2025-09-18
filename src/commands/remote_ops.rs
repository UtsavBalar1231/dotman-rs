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
