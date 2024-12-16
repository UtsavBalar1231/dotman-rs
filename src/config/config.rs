use crate::{
    config::{
        config_cache::{CacheEntry, ConfigCacheSerde},
        config_entry::{ConfType, ConfigEntry},
        dotconfig_path::DotconfigPath,
    },
    errors::ConfigError,
    file_manager, hasher,
};
use dashmap::DashMap;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::{fmt, fs, io::Write, path, sync};

#[derive(Deserialize, Serialize, Clone)]
pub struct Config {
    #[serde(skip)]
    path: path::PathBuf,
    #[serde(skip)]
    hash_cache: sync::Arc<DashMap<path::PathBuf, CacheEntry>>,
    pub dotconfigs_path: DotconfigPath,
    pub configs: Vec<ConfigEntry>,
}

impl fmt::Display for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", toml::to_string_pretty(self).unwrap())
    }
}

impl Config {
    pub fn get_config_path(config_path: Option<&str>) -> Result<path::PathBuf, ConfigError> {
        let config_path_name = format!("{}/config.toml", env!("CARGO_PKG_NAME"));

        if let Some(path) = config_path {
            return Ok(path::PathBuf::from(path));
        }

        if let Ok(path) = std::env::var("DOTMAN_CONFIG_PATH") {
            // Check if path is valid
            let path = path::PathBuf::from(path);
            if path.exists() {
                return Ok(path);
            } else {
                eprintln!(
                    "Config file set in $DOTMAN_CONFIG_PATH, but not found: {}",
                    path.display()
                );

                return Err(ConfigError::ConfigFileNotFound(path));
            }
        }

        Ok(dirs::config_dir()
            .unwrap_or_else(|| dirs::home_dir().unwrap())
            .join(config_path_name))
    }

    pub fn new(path: path::PathBuf, dotconfigs_path: DotconfigPath) -> Self {
        Self {
            path,
            dotconfigs_path,
            hash_cache: sync::Arc::new(DashMap::new()),
            configs: Vec::new(),
        }
    }

    fn load_hash_cache(&mut self) -> Result<(), ConfigError> {
        let cache_path = get_cache_path(&self.path);

        if cache_path.exists() {
            let content = fs::read_to_string(cache_path)?;
            let serde_wrapper: ConfigCacheSerde = toml::from_str(&content).unwrap_or_default();
            self.hash_cache = sync::Arc::new(serde_wrapper.to_dashmap());
        } else {
            self.hash_cache = sync::Arc::new(DashMap::new());
        }

        Ok(())
    }

    fn save_hash_cache(&self) -> Result<(), ConfigError> {
        if self.hash_cache.is_empty() {
            return Ok(());
        }

        let cache_path = get_cache_path(&self.path);
        if !cache_path.exists() {
            fs::create_dir_all(cache_path.parent().unwrap())?;
        }

        let serde_wrapper = ConfigCacheSerde::from_dashmap(&self.hash_cache);
        let content = toml::to_string_pretty(&serde_wrapper)?;
        fs::write(cache_path, content)?;

        Ok(())
    }

    pub fn pull_config(&mut self, clean: bool) -> Result<Option<path::PathBuf>, ConfigError> {
        self.load_hash_cache()?;

        let tracking_path = &self.dotconfigs_path.get_path();

        if !tracking_path.exists() {
            fs::create_dir_all(tracking_path)?;
        }

        let mut copied_path = None;

        // Parallelize the processing of each config entry
        let copied_paths: Vec<Option<path::PathBuf>> = self
            .configs
            .par_iter_mut()
            .map(|entry| {
                let src_path = &entry.path;
                let current_hash = if src_path.is_dir() {
                    hasher::get_complete_dir_hash(&src_path, &self.hash_cache).unwrap_or_default()
                } else if src_path.is_file() {
                    hasher::get_file_hash(&src_path, &self.hash_cache).unwrap_or_default()
                } else {
                    return Err(ConfigError::InvalidPath(format!(
                        "Path is not a file or directory: {}",
                        src_path.display()
                    )));
                };

                // Ensure the destination path considers the file extension dynamically
                let dest_path = if src_path.is_file() {
                    tracking_path.join(src_path.file_name().unwrap())
                } else {
                    tracking_path.join(&entry.name)
                };

                // Compare and pull if hashes differ
                if entry.hash != current_hash
                    || clean
                    || self.hash_cache.is_empty()
                    || entry.hash.is_empty()
                    || !dest_path.exists()
                {
                    println!(
                        "Pulling {}: {}",
                        if src_path.is_file() { "file" } else { "dir" },
                        src_path.file_name().unwrap().to_string_lossy()
                    );

                    file_manager::fs_copy_recursive(&src_path, &&dest_path)?;
                    entry.hash = current_hash;
                    Ok(Some(dest_path))
                } else {
                    println!("No changes detected for: {}", entry.name);
                    Ok(None)
                }
            })
            .collect::<Result<Vec<_>, ConfigError>>()?;

        if let Some(path) = copied_paths.into_iter().find(|path| path.is_some()) {
            copied_path = path;
        }

        self.save_hash_cache()?;
        self.save_config()?;

        Ok(copied_path)
    }

    pub fn push_config(&self, clean: bool) -> Result<(), ConfigError> {
        let tracking_path = &self.dotconfigs_path.get_path();

        if !tracking_path.exists() {
            fs::create_dir_all(tracking_path)?;
        }

        for entry in &self.configs {
            // Dynamically derive the correct filename and extension from entry.path
            let file_name = entry.path.file_name().ok_or_else(|| {
                ConfigError::InvalidPath(format!(
                    "Failed to extract file name from path: {:?}",
                    entry.path
                ))
            })?;

            let src_path = if entry.path.exists() {
                &entry.path
            } else {
                &tracking_path.join(file_name)
            };
            let dst_path = &entry.path;

            if clean {
                file_manager::fs_remove_recursive(dst_path)?;
            }

            if !src_path.exists() {
                return Err(ConfigError::InvalidPath(format!(
                    "Source file does not exist: {:?}",
                    src_path
                )));
            }

            // Perform the copy operation
            println!("Pushing from {:?} to {:?}", src_path, dst_path);
            file_manager::fs_copy_recursive(src_path, dst_path)?;
        }

        self.save_config()
    }

    pub fn add_config(&mut self, name: &str, path: &str) -> Result<(), ConfigError> {
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
            hash: hasher::get_complete_dir_hash(&config_path, &self.hash_cache).unwrap_or_default(),
            conf_type: ConfType::get_conf_type(&config_path),
        };

        self.configs.push(new_entry.clone());

        println!("Added config: {}", new_entry);
        self.save_config()
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
        let tracking_path = &self.dotconfigs_path.get_path();
        // Delete all the configs in the tracking directory
        file_manager::fs_remove_recursive(tracking_path)?;
        // Recreate the tracking directory
        fs::create_dir_all(tracking_path)?;

        Ok(())
    }
}

fn get_cache_path(original_path: &path::Path) -> path::PathBuf {
    let path_buf = original_path
        .to_string_lossy()
        .to_string()
        .replace(".toml", "_cache.toml");

    path::PathBuf::from(path_buf)
}

#[cfg(test)]
mod tests {
    use fs::File;
    use tempfile::tempdir;

    use super::*;

    #[test]
    fn test_config_get_config_path_with_env() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("config.toml");
        File::create(&config_path).unwrap();

        std::env::set_var(
            "DOTMAN_CONFIG_PATH",
            config_path.to_str().expect("Unable to get config path"),
        );
        let path = Config::get_config_path(None);

        assert!(path.is_ok());
        let path = path.unwrap();
        assert_eq!(path, config_path);
        std::env::remove_var("DOTMAN_CONFIG_PATH");
    }

    #[test]
    fn test_config_get_config_path_with_arg() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("my_config.toml");
        File::create(&config_path).unwrap();

        let path = Config::get_config_path(Some(config_path.to_str().unwrap()));

        assert!(path.is_ok());
        let path = path.unwrap();
        assert_eq!(path, config_path);
    }

    #[test]
    fn test_config_new() {
        let temp_dir = tempdir().unwrap();
        let dotconfigs_path = DotconfigPath::Local(temp_dir.path().to_path_buf());
        let config_path = temp_dir.path().join("config.toml");
        File::create(&config_path).unwrap();

        let config = Config::new(config_path, dotconfigs_path.clone());
        assert_eq!(config.configs.len(), 0);
        assert_eq!(
            config.dotconfigs_path.get_path(),
            dotconfigs_path.get_path()
        );
    }

    #[test]
    fn test_config_pull_push_config() {
        // Create a temporary directory for dotfiles tracking
        let temp_dir = tempdir().unwrap();
        let dotconfigs_path = DotconfigPath::Local(temp_dir.path().join("dotfiles"));
        fs::create_dir_all(&dotconfigs_path.get_path()).unwrap();

        let config_path = temp_dir.path().join("config.toml");
        File::create(&config_path).unwrap();

        let mut config = Config::new(config_path.clone(), dotconfigs_path);
        let test_file_path = temp_dir.path().join("test_file.txt");
        let mut test_file = File::create(&test_file_path).unwrap();
        writeln!(test_file, "test content").unwrap();

        config
            .add_config("test_file", test_file_path.to_str().unwrap())
            .unwrap();

        config.clear_config().unwrap();
        config.pull_config(false).unwrap();
        let pulled_file = config.dotconfigs_path.get_path().join("test_file.txt");
        assert!(pulled_file.exists());

        let new_test_file_path = temp_dir.path().join("new_test_file.txt");
        File::create(&new_test_file_path).unwrap();
        config.configs[0].path = new_test_file_path.clone();

        Config::print_config(Some(&config)).unwrap();

        config.push_config(false).unwrap();

        assert!(new_test_file_path.exists());
    }

    #[test]
    fn test_config_clear_config() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("config.toml");
        File::create(&config_path).unwrap();
        let dotconfigs_path = DotconfigPath::Local(temp_dir.path().to_path_buf());
        fs::create_dir_all(&dotconfigs_path.get_path()).unwrap();
        let mut config = Config::new(config_path.clone(), dotconfigs_path);
        let test_file_path = temp_dir.path().join("test_file.txt");
        File::create(&test_file_path).unwrap();
        config
            .add_config("test_file", test_file_path.to_str().unwrap())
            .unwrap();
        config.pull_config(false).unwrap();
        let loaded_config = Config::load_config(&config_path).unwrap();
        assert!(!loaded_config.configs[0].hash.is_empty());
        loaded_config.clear_config().unwrap();
        let loaded_config = Config::load_config(&config_path).unwrap();
        assert!(loaded_config.configs[0].hash.is_empty());
    }
    #[test]
    fn test_config_add_config() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("config.toml");
        let dotconfigs_path = DotconfigPath::Local(temp_dir.path().to_path_buf());
        fs::create_dir_all(&dotconfigs_path.get_path()).unwrap();

        let mut config = Config::new(config_path.clone(), dotconfigs_path);
        let config_len_before = config.configs.len();

        let test_file_path = temp_dir.path().join("test_file.txt");
        File::create(&test_file_path).unwrap();

        config
            .add_config("test_file", test_file_path.to_str().unwrap())
            .unwrap();

        // Now check the *same* config instance
        assert_eq!(config.configs.len(), config_len_before + 1);
        assert_eq!(config.configs[0].name, "test_file");

        let result = config.add_config("test_file", test_file_path.to_str().unwrap());
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Config with name test_file already exists"));

        let non_existent_path = temp_dir.path().join("non_existent_file.txt");
        let result = config.add_config("non_existent", non_existent_path.to_str().unwrap());
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Path does not exist"));
    }

    #[test]
    fn test_config_load_save_config() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("config.toml");
        let dotconfigs_path = DotconfigPath::Local(temp_dir.path().join("dotfiles"));
        fs::create_dir_all(&dotconfigs_path.get_path()).unwrap();
        let config = Config::new(config_path.clone(), dotconfigs_path);
        config.save_config().unwrap();
        let loaded_config = Config::load_config(&config_path).unwrap();
        assert_eq!(config.configs.len(), loaded_config.configs.len());
    }
}
