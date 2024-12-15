use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::{Path, PathBuf};

#[derive(Clone, Deserialize, Serialize)]
pub struct ConfigEntry {
    pub name: String,
    pub path: PathBuf,
    pub hash: String,
    pub conf_type: ConfType,
}

#[derive(Clone, Deserialize, Serialize)]
pub enum ConfType {
    Dir,
    File,
    Unknown,
}

impl ConfType {
    pub fn get_conf_type(path: &Path) -> Self {
        if path.is_dir() {
            ConfType::Dir
        } else if path.is_file() {
            ConfType::File
        } else {
            ConfType::Unknown
        }
    }
}

impl ConfigEntry {
    pub fn new(name: String, path: PathBuf, hash: String, conf_type: ConfType) -> Self {
        Self {
            name,
            path,
            hash,
            conf_type,
        }
    }
}

impl fmt::Display for ConfType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                ConfType::Dir => "Dir",
                ConfType::File => "File",
                ConfType::Unknown => "Unknown",
            }
        )
    }
}

impl fmt::Display for ConfigEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", toml::to_string_pretty(self).unwrap())
    }
}
