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

        // Process each config entry in parallel
        let copied_paths: Vec<Option<path::PathBuf>> = self
            .configs
            .par_iter_mut()
            .map(|entry| {
                let src_path = &entry.path;

                // Determine the destination path
                let dest_path = if entry.conf_type == ConfType::File {
                    tracking_path.join(&entry.name) // Copy file directly to `tracking_path/name`
                } else {
                    tracking_path.join(&entry.name) // Copy directory to `tracking_path/name`
                };

                // Get the current hash based on file or directory type
                let current_hash = match src_path.metadata() {
                    Ok(metadata) if metadata.is_file() => {
                        hasher::get_file_hash(src_path, &self.hash_cache).unwrap_or_default()
                    }
                    Ok(metadata) if metadata.is_dir() => {
                        hasher::get_complete_dir_hash(src_path, &self.hash_cache)
                            .unwrap_or_default()
                    }
                    _ => {
                        return Err(ConfigError::InvalidPath(format!(
                            "Path is not a valid file or directory: {}",
                            src_path.display()
                        )));
                    }
                };

                // Check if copying is needed
                let needs_pull = entry.hash != current_hash
                    || clean
                    || self.hash_cache.is_empty()
                    || entry.hash.is_empty()
                    || !dest_path.exists();

                if needs_pull {
                    println!(
                        "Pulling {}: {}",
                        if src_path.is_file() { "file" } else { "dir" },
                        src_path.display()
                    );

                    // Clean the destination if required
                    if clean {
                        file_manager::fs_remove_recursive(&dest_path)?;
                    }

                    // Recursively copy to destination
                    file_manager::fs_copy_recursive(src_path, &dest_path)?;

                    entry.hash = current_hash;
                    Ok(Some(dest_path))
                } else {
                    println!("No changes detected for: {}", entry.name);
                    Ok(None)
                }
            })
            .collect::<Result<Vec<_>, ConfigError>>()?;

        // Collect the first non-None copied path
        let copied_path = copied_paths.into_iter().flatten().next();

        // Save updated configurations and hash cache
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
            let name_based_src = tracking_path.join(&entry.name);
            let fallback_src = tracking_path.join(entry.path.file_name().ok_or_else(|| {
                ConfigError::InvalidPath(format!(
                    "Failed to extract file name from path: {:?}",
                    entry.path
                ))
            })?);

            // Determine the actual source path to use
            let src_path = if name_based_src.exists() {
                name_based_src
            } else {
                fallback_src
            };

            // Destination path is the original path from the configuration entry
            let dst_path = &entry.path;

            println!("SRC PATH: {}", src_path.display());
            println!("DST PATH: {}", dst_path.display());

            // If `clean` is true or if overwrite support is enabled, remove the destination path
            if clean || dst_path.exists() {
                file_manager::fs_remove_recursive(dst_path)?;
            }

            // Ensure the source path exists before attempting to copy
            if !src_path.exists() {
                return Err(ConfigError::InvalidPath(format!(
                    "Source path does not exist: {:?}",
                    src_path
                )));
            }

            println!("Pushing from {:?} to {:?}", src_path, dst_path);

            file_manager::fs_copy_recursive(&src_path, dst_path)?;
        }

        self.save_config()
    }

    pub fn add_config(&mut self, name: &str, path: &str) -> Result<(), ConfigError> {
        // Expand the path to its absolute form
        let expanded_path = Self::expand_path(path)?;

        if self
            .configs
            .iter()
            .any(|c| c.name == name || c.path.to_str().unwrap() == path)
        {
            return Err(ConfigError::InvalidConfig(format!(
                "Config with name {} already exists",
                name
            )));
        }

        if !expanded_path.exists() {
            return Err(ConfigError::InvalidPath(format!(
                "Path does not exist: {}",
                expanded_path.display()
            )));
        }

        let new_entry = ConfigEntry {
            name: name.to_string(),
            path: expanded_path.clone(),
            hash: hasher::get_complete_dir_hash(&expanded_path, &self.hash_cache)
                .unwrap_or_default(),
            conf_type: ConfType::get_conf_type(&expanded_path),
        };

        self.configs.push(new_entry.clone());

        println!("Added config: {}", new_entry);
        self.save_config()
    }

    // Helper function to expand the path
    fn expand_path(path_str: &str) -> Result<path::PathBuf, ConfigError> {
        // Handle ~ (home directory)
        let path_str = if path_str.starts_with("~/") {
            match dirs::home_dir() {
                Some(home) => path_str.replacen("~", &home.to_string_lossy(), 1),
                None => {
                    return Err(ConfigError::InvalidPath(
                        "Could not determine home directory".to_string(),
                    ))
                }
            }
        } else {
            path_str.to_string()
        };

        // Handle environment variables
        let path_str = shellexpand::env(&path_str)
            .map_err(|e| {
                ConfigError::InvalidPath(format!("Failed to expand environment variables: {}", e))
            })?
            .to_string();

        // Convert to absolute path
        let abs_path = path::Path::new(&path_str)
            .canonicalize()
            .map_err(|e| ConfigError::InvalidPath(format!("Failed to canonicalize path: {}", e)))?;

        Ok(abs_path)
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
        let pulled_file = config.dotconfigs_path.get_path().join("test_file");
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
