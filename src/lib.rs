pub mod commands;
pub mod config;
pub mod storage;
pub mod utils;

use anyhow::Result;
use std::path::PathBuf;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const DEFAULT_REPO_DIR: &str = ".dotman";
pub const DEFAULT_CONFIG_PATH: &str = ".config/dotman/config";
pub const INDEX_FILE: &str = "index.bin";
pub const COMMITS_DIR: &str = "commits";
pub const OBJECTS_DIR: &str = "objects";

#[derive(Debug, Clone)]
pub struct DotmanContext {
    pub repo_path: PathBuf,
    pub config_path: PathBuf,
    pub config: config::Config,
}

impl DotmanContext {
    pub fn new() -> Result<Self> {
        let home =
            dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;
        let config_path = home.join(DEFAULT_CONFIG_PATH);
        let config = config::Config::load(&config_path)?;
        let repo_path = config.core.repo_path.clone();

        Ok(Self {
            repo_path,
            config_path,
            config,
        })
    }

    pub fn ensure_repo_exists(&self) -> Result<()> {
        std::fs::create_dir_all(&self.repo_path)?;
        std::fs::create_dir_all(self.repo_path.join(COMMITS_DIR))?;
        std::fs::create_dir_all(self.repo_path.join(OBJECTS_DIR))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constants() {
        assert_eq!(DEFAULT_REPO_DIR, ".dotman");
        assert_eq!(INDEX_FILE, "index.bin");
    }
}
