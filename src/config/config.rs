use crate::{
    config::{
        config_entry::{ConfType, ConfigEntry},
        dotconfig_path::DotconfigPath,
    },
    errors::ConfigError,
    file_manager, hasher,
};
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use std::{collections::HashMap, ffi};
use std::{fmt, fs, io::Write, path};

#[derive(Deserialize, Serialize, Clone)]
pub struct Config {
    #[serde(skip)]
    path: path::PathBuf,
    #[serde(skip)]
    hash_cache: Option<HashMap<path::PathBuf, String>>,
    pub dotconfigs_path: DotconfigPath,
    pub configs: Vec<ConfigEntry>,
}

impl fmt::Display for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", toml::to_string_pretty(self).unwrap())
    }
}

impl Config {
    pub fn get_config_path(config_path: Option<String>) -> path::PathBuf {
        let config_path_name = format!("{}/config.toml", env!("CARGO_PKG_NAME"));

        if let Some(path) = config_path {
            return path::PathBuf::from(path);
        }

        if let Ok(path) = std::env::var("DOTMAN_CONFIG_PATH") {
            // Check if path is valid
            let path = path::PathBuf::from(path);
            if path.exists() {
                return path;
            } else {
                eprintln!(
                    "Config file set in $DOTMAN_CONFIG_PATH, but not found: {}",
                    path.display()
                );
                eprintln!("Using default path: {}", config_path_name);
            }
        }

        dirs::config_dir()
            .unwrap_or_else(|| dirs::home_dir().unwrap())
            .join(config_path_name)
    }

    pub fn new(path: path::PathBuf, dotconfigs_path: DotconfigPath) -> Self {
        let mut hasher = Sha1::new();
        let default_config = ConfigEntry::new(
            dotconfigs_path
                .get_path()
                .file_name()
                .unwrap_or_else(|| ffi::OsStr::new("<config_name>"))
                .to_string_lossy()
                .to_string(),
            dotconfigs_path.get_path(),
            hasher::get_complete_dir_hash(&dotconfigs_path.get_path(), &mut hasher, &mut None)
                .unwrap_or_default(),
            ConfType::Dir,
        );

        Self {
            path,
            dotconfigs_path,
            hash_cache: None,
            configs: Vec::from([default_config]),
        }
    }

    fn load_hash_cache(&mut self) -> Result<(), ConfigError> {
        let cache_path = self.path.with_extension("_cache.toml");
        if cache_path.exists() {
            let content = fs::read_to_string(cache_path)?;
            self.hash_cache = toml::from_str(&content).unwrap_or_default();
        } else {
            self.hash_cache = None;
        }
        Ok(())
    }

    fn save_hash_cache(&self) -> Result<(), ConfigError> {
        let cache_path = self.path.with_extension("_cache.toml");
        let content = toml::to_string_pretty(&self.hash_cache)?;
        fs::write(cache_path, content)?;
        Ok(())
    }

    pub fn pull_config(&mut self, clean: bool) -> Result<(), ConfigError> {
        self.load_hash_cache()?;
        let backup_path = &self.dotconfigs_path.get_path();

        if !backup_path.exists() {
            fs::create_dir_all(backup_path)?;
        }

        let mut hasher = Sha1::new();
        for entry in &mut self.configs {
            let src_path = entry.path.clone();
            let current_hash = if src_path.is_dir() {
                hasher::get_complete_dir_hash(&src_path, &mut hasher, &mut self.hash_cache)
                    .unwrap_or_default()
            } else if src_path.is_file() {
                let mut hasher = Sha1::new();
                hasher::get_file_hash(&src_path, &mut hasher, &mut self.hash_cache)
                    .unwrap_or_default()
            } else {
                String::new()
            };

            // Compare and pull if hashes differ
            if entry.hash != current_hash || clean {
                println!("Pulling: {}", entry.name);
                let dest_path = backup_path.join(&entry.name);
                file_manager::fs_copy_recursive(&src_path, &dest_path)?;
                entry.hash = current_hash;
            } else {
                println!("No changes detected for: {}", entry.name);
            }
        }

        self.save_hash_cache()?;
        self.save_config()
    }

    pub fn push_config(&self, clean: bool) -> Result<(), ConfigError> {
        let backup_path = &self.dotconfigs_path.get_path();

        if !backup_path.exists() {
            fs::create_dir_all(backup_path)?;
        }

        for entry in &self.configs {
            let src_path = &backup_path.join(&entry.name);
            let dst_path = &entry.path;

            if clean {
                file_manager::fs_remove_recursive(dst_path)?;
            }

            println!("Pushing: {}", entry.name);
            file_manager::fs_copy_recursive(&src_path, &dst_path)?;
        }

        self.save_config()
    }

    pub fn add_config(&self, name: &str, path: &str) -> Result<(), ConfigError> {
        let mut hasher = Sha1::new();
        let config_path = path::PathBuf::from(path);

        if self.configs.iter().any(|c| c.name == name) {
            return Err(ConfigError::InvalidConfig(format!(
                "Config with name {} already exists",
                name
            )));
        }

        if !config_path.exists() {
            return Err(ConfigError::InvalidPath(format!(
                "Path does not exist: {}",
                path
            )));
        }

        let new_entry = ConfigEntry {
            name: name.to_string(),
            path: config_path.to_path_buf(),
            hash: hasher::get_complete_dir_hash(&config_path, &mut hasher, &mut None)
                .unwrap_or_default(),
            conf_type: ConfType::get_conf_type(&config_path),
        };

        let mut updated_config = self.clone();
        updated_config.configs.push(new_entry);

        updated_config.save_config()
    }

    pub fn edit_config(&self) -> Result<(), ConfigError> {
        let editor = std::env::var("EDITOR").unwrap_or_else(|_| "nano".to_string());
        std::process::Command::new(editor)
            .arg(&self.path)
            .status()?;
        Ok(())
    }

    pub fn fix_config(&self) -> Result<(), ConfigError> {
        let valid_configs: Vec<_> = self
            .configs
            .iter()
            .filter(|entry| entry.path.exists())
            .cloned()
            .collect();

        let fixed_config = Self {
            path: self.path.clone(),
            dotconfigs_path: self.dotconfigs_path.clone(),
            hash_cache: self.hash_cache.clone(),
            configs: valid_configs,
        };

        fixed_config.save_config()
    }

    fn save_config(&self) -> Result<(), ConfigError> {
        println!("Saving config: {}", self.path.display());

        let mut serialized = toml::to_string_pretty(self)?;

        let home_dir = dirs::home_dir().unwrap_or_else(|| path::PathBuf::from("/"));

        // Replace occurrences of the actual home directory with $HOME
        serialized = serialized.replace(&*home_dir.to_string_lossy(), "$HOME");

        let mut file = fs::File::create(&self.path)?;
        file.write_all(serialized.as_bytes())?;

        Ok(())
    }

    pub fn clear_config(&self) -> Result<(), ConfigError> {
        let mut cleared_config = self.clone();

        for entry in &mut cleared_config.configs {
            entry.hash = String::new();
        }

        println!("{}", cleared_config);

        cleared_config.save_config()
    }

    pub fn print_config(config: Option<&Config>) -> Result<(), ConfigError> {
        if let Some(config) = config {
            println!("{}", config);
        } else {
            let cwd = std::env::current_dir().unwrap_or_else(|_| path::PathBuf::from("."));
            let new_config = Config::new(path::PathBuf::new(), DotconfigPath::Local(cwd));
            let toml_config = toml::to_string_pretty(&new_config)?;
            println!("{}", toml_config);
        }
        Ok(())
    }

    pub fn load_config(config_path: &path::Path) -> Result<Config, ConfigError> {
        let mut content = fs::read_to_string(config_path)?;
        let home_dir = dirs::home_dir().unwrap();
        let replacements = [
            ("~", &home_dir.to_string_lossy()),
            ("$HOME", &home_dir.to_string_lossy()),
        ];

        content = replacements
            .iter()
            .fold(content, |acc, &(from, to)| acc.replace(from, to));

        let mut config: Config = toml::from_str(&content)?;

        config.path = config_path.to_path_buf();

        Ok(config)
    }

    pub fn clean_configs(&mut self) -> Result<(), ConfigError> {
        // Delete all the configs in the tracking directory
        file_manager::fs_remove_recursive(self.dotconfigs_path.get_path())?;

        Ok(())
    }
}
