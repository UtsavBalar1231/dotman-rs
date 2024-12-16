use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone)]
pub enum DotconfigPath {
    Github(String),
    Local(PathBuf),
}

impl DotconfigPath {
    pub fn get_path(&self) -> PathBuf {
        match self {
            DotconfigPath::Local(local_dotconfigs_path) => local_dotconfigs_path.clone(),
            DotconfigPath::Github(remote_dotconfigs_path) => PathBuf::from(remote_dotconfigs_path),
        }
    }
}

impl fmt::Display for DotconfigPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DotconfigPath::Local(local_dotconfigs_path) => {
                write!(f, "{}", local_dotconfigs_path.display())
            }
            DotconfigPath::Github(remote_dotconfigs_path) => {
                write!(f, "{remote_dotconfigs_path}")
            }
        }
    }
}

impl fmt::Debug for DotconfigPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DotconfigPath::Local(local_dotconfigs_path) => {
                write!(f, "{}", local_dotconfigs_path.display())
            }
            DotconfigPath::Github(remote_dotconfigs_path) => {
                write!(f, "{remote_dotconfigs_path}")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use fs::File;
    use tempfile::tempdir;

    use crate::config::Config;

    use super::*;

    #[test]
    fn test_dotconfig_path_display() {
        let local_path = DotconfigPath::Local(PathBuf::from("/tmp/test"));
        assert_eq!(local_path.to_string(), "/tmp/test");

        let github_path = DotconfigPath::Github("github.com/test/repo".to_string());
        assert_eq!(github_path.to_string(), "github.com/test/repo");
    }

    #[test]
    fn test_dotconfig_path_debug() {
        let local_path = DotconfigPath::Local(PathBuf::from("/tmp/test"));
        assert_eq!(format!("{:?}", local_path), "/tmp/test");

        let github_path = DotconfigPath::Github("github.com/test/repo".to_string());
        assert_eq!(format!("{:?}", github_path), "github.com/test/repo");
    }

    #[test]
    fn test_config_clean_configs() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("config.toml");
        let dotconfigs_path = DotconfigPath::Local(temp_dir.path().join("dotfiles"));
        fs::create_dir_all(&dotconfigs_path.get_path()).unwrap();

        let mut config = Config::new(config_path, dotconfigs_path.clone());

        let test_file_path = temp_dir.path().join("test_file.txt");
        File::create(&test_file_path).unwrap();

        config
            .add_config("test_file", test_file_path.to_str().unwrap())
            .unwrap();

        config.pull_config(false).unwrap();

        let check_path = dotconfigs_path.get_path().join("test_file.txt");
        println!("{}", check_path.display());

        assert!(&check_path.exists());
        config.clean_configs().unwrap();
        assert!(!&check_path.exists());
    }
}
