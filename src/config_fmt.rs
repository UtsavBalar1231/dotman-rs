use std::fmt;
use crate::config::{ConfType, ConfigEntry, DotconfigPath, Config};

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
        write!(
            f,
            "(name: {:?}, path: {:?}, hash: {:?}, conf_type: {})",
            self.name,
            self.path.display(),
            self.hash,
            self.conf_type
        )
    }
}

impl fmt::Display for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "(\n    dotconfigs_path: {:?},\n    configs: [\n",
            self.dotconfigs_path
        )?;
        for config in &self.configs {
            writeln!(f, "        {},", config)?;
        }
        write!(f, "    ]\n)")
    }
}


