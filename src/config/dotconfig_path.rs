use std::fmt;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};

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


