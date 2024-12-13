use crate::{hasher, utils};
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use std::path::{Path, PathBuf};
use std::{
    fs,
    {io, io::Write},
};

#[derive(Clone, Deserialize, Serialize)]
pub struct ConfigEntry {
    pub name: String,
    pub path: PathBuf,
    pub hash: String,
    pub conf_type: ConfType,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct Config {
    #[serde(skip_serializing, skip_deserializing)]
    path: PathBuf,
    pub dotconfigs_path: DotconfigPath,
    pub configs: Vec<ConfigEntry>,
}

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

#[derive(Clone, Deserialize, Serialize)]
pub enum ConfType {
    Dir,
    File,
    Unknown,
}

impl ConfType {
    fn get_conf_type(path: &Path) -> Self {
        if path.is_dir() {
            ConfType::Dir
        } else if path.is_file() {
            ConfType::File
        } else {
            ConfType::Unknown
        }
    }
}

impl Config {
    pub fn new(path: PathBuf, dotconfigs_path: DotconfigPath) -> Self {
        let mut hasher = Sha1::new();
        let default_config = ConfigEntry::new(
            dotconfigs_path.to_string(),
            dotconfigs_path.get_path(),
            hasher::get_complete_dir_hash(&dotconfigs_path.get_path(), &mut hasher)
                .unwrap_or_default(),
            ConfType::Dir,
        );

        Self {
            path,
            dotconfigs_path,
            configs: Vec::from([default_config]),
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
pub struct PrintConfigOpts {
    pub new_required: bool,
}

impl PrintConfigOpts {
    pub fn new_required() -> Self {
        Self { new_required: true }
    }

    pub fn default() -> Self {
        Self {
            new_required: false,
        }
    }
}
impl Config {
    pub fn pull_config(&mut self, clean: bool) -> io::Result<()> {
        let backup_path = &self.dotconfigs_path.get_path();

        if !backup_path.exists() {
            fs::create_dir_all(backup_path)?;
        }

        for entry in &mut self.configs {
            let src_path = entry.path.clone();
            let current_hash = if src_path.is_dir() {
                let mut hasher = Sha1::new();
                hasher::get_complete_dir_hash(&src_path, &mut hasher).unwrap_or_default()
            } else if src_path.is_file() {
                let mut hasher = Sha1::new();
                hasher::get_file_hash(&src_path, &mut hasher).unwrap_or_default()
            } else {
                String::new()
            };

            // Compare and pull if hashes differ
            if entry.hash != current_hash || clean {
                println!("Pulling: {}", entry.name);
                let dest_path = backup_path.join(&entry.name);
                utils::copy_recursive(&src_path, &dest_path)?;
                entry.hash = current_hash; // Update stored hash
            } else {
                println!("No changes detected for: {}", entry.name);
            }
        }

        self.save_config()
    }

    pub fn push_config(&self, clean: bool) -> io::Result<()> {
        let backup_path = &self.dotconfigs_path.get_path();
        if !backup_path.exists() {
            fs::create_dir_all(backup_path)?;
        }

        for entry in &self.configs {
            let src_path = &backup_path.join(&entry.name);
            let dst_path = entry.path.clone();

            if clean {
                if dst_path.exists() {
                    if dst_path.is_dir() {
                        fs::remove_dir_all(&dst_path)?;
                    } else {
                        fs::remove_file(&dst_path)?;
                    }
                }
            }

            println!("Pushing: {}", entry.name);
            utils::copy_recursive(&src_path, &dst_path)?;
        }

        self.save_config()
    }

    pub fn add_config(&self, name: &str, path: &str) -> io::Result<()> {
        let mut hasher = Sha1::new();
        let new_entry = ConfigEntry {
            name: name.to_string(),
            path: PathBuf::from(path),
            hash: hasher::get_complete_dir_hash(&PathBuf::from(path), &mut hasher)
                .unwrap_or_default(),
            conf_type: ConfType::get_conf_type(&PathBuf::from(path)),
        };

        let mut updated_config = self.clone();
        updated_config.configs.push(new_entry);

        updated_config.save_config()
    }

    pub fn edit_config(&self) -> io::Result<()> {
        let editor = std::env::var("EDITOR").unwrap_or_else(|_| "nano".to_string());
        std::process::Command::new(editor)
            .arg(&self.path)
            .status()?;
        Ok(())
    }

    pub fn fix_config(&self) -> io::Result<()> {
        let valid_configs: Vec<_> = self
            .configs
            .iter()
            .filter(|entry| entry.path.exists())
            .cloned()
            .collect();

        let fixed_config = Self {
            path: self.path.clone(),
            dotconfigs_path: self.dotconfigs_path.clone(),
            configs: valid_configs,
        };

        fixed_config.save_config()
    }

    fn save_config(&self) -> io::Result<()> {
        println!("Saving config: {}", self.path.display());
        let ron_pretty = utils::get_ron_formatter();

        let mut serialized = ron::ser::to_string_pretty(self, ron_pretty).map_err(|err| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Failed to serialize config: {}", err),
            )
        })?;

        let home_dir = dirs::home_dir().unwrap();

        // Replace occurrences of the actual home directory with $HOME
        serialized = serialized.replace(&*home_dir.to_string_lossy(), "$HOME");

        let mut file = fs::File::create(&self.path)?;
        file.write_all(serialized.as_bytes())
    }

    pub fn clear_config(&self) -> Result<(), io::Error> {
        let mut cleared_config = self.clone();

        for entry in &mut cleared_config.configs {
            entry.hash = String::new();
        }

        println!("{}", cleared_config);

        cleared_config.save_config()
    }

    pub fn print_config(config: Option<&Config>, opts: PrintConfigOpts) -> io::Result<()> {
        if opts.new_required {
            let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::new());
            let new_config = Config::new(PathBuf::new(), DotconfigPath::Local(cwd));
            // Pretty ron output
            let ron_config =
                ron::ser::to_string_pretty(&new_config, ron::ser::PrettyConfig::default())
                    .map_err(|err| {
                        io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!("Failed to serialize config: {}", err),
                        )
                    })?;
            println!("{}", ron_config);
        } else {
            println!("{}", config.unwrap());
        }
        Ok(())
    }

    pub fn load_config(config_path: &Path) -> io::Result<Config> {
        let mut content = fs::read_to_string(config_path)?;
        let home_dir = dirs::home_dir().unwrap();

        // Replace ~ and $HOME with the actual home directory path
        content = content.replace("~", &home_dir.to_string_lossy());
        if content.contains("$HOME") {
            content = content.replace("$HOME", &home_dir.to_string_lossy());
        }

        let mut config: Config = ron::from_str(&content).map_err(|err| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Failed to parse config: {}", err),
            )
        })?;
        config.path = config_path.to_path_buf();

        Ok(config)
    }
}
