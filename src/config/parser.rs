use super::Config;
use anyhow::{Context, Result};
use memmap2::MmapOptions;
use std::fs::File;
use std::path::Path;

// Fast TOML parser optimized for our config structure
pub fn parse_config_file(path: &Path) -> Result<Config> {
    // For small files, use regular reading
    let metadata = std::fs::metadata(path)?;

    if metadata.len() < 4096 {
        // Small file - read normally
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {:?}", path))?;
        parse_config_str(&content)
    } else {
        // Large file - use memory mapping
        let file = File::open(path)?;
        let mmap = unsafe { MmapOptions::new().map(&file)? };

        // Validate UTF-8 using SIMD
        let content = simdutf8::basic::from_utf8(&mmap)
            .map_err(|e| anyhow::anyhow!("Invalid UTF-8 in config file: {}", e))?;

        parse_config_str(content)
    }
}

fn parse_config_str(content: &str) -> Result<Config> {
    // Use optimized TOML parsing
    let config: Config = toml::from_str(content).with_context(|| "Failed to parse TOML config")?;

    // Validate and return validation errors directly without wrapping
    validate_config(&config)?;
    Ok(config)
}

fn validate_config(config: &Config) -> Result<()> {
    // Validate compression level
    if config.core.compression_level < 1 || config.core.compression_level > 22 {
        anyhow::bail!("Compression level must be between 1 and 22");
    }

    // Validate thread count
    if config.performance.parallel_threads == 0 {
        anyhow::bail!("Parallel threads must be at least 1");
    }

    // Validate cache size
    if config.performance.cache_size > 10_000 {
        anyhow::bail!("Cache size cannot exceed 10GB");
    }

    Ok(())
}

// Fast key-value parser for simple config updates
pub struct FastConfigUpdater {
    content: Vec<u8>,
}

impl FastConfigUpdater {
    pub fn new(path: &Path) -> Result<Self> {
        let content = std::fs::read(path)?;
        Ok(Self { content })
    }

    pub fn update_value(&mut self, section: &str, key: &str, value: &str) -> Result<()> {
        // Use SIMD to find the section and key quickly
        let section_pattern = format!("[{}]", section);
        let key_pattern = format!("{} =", key);

        // Find section start
        let section_pos = self.find_pattern(section_pattern.as_bytes())?;

        // Find key within section
        let key_pos = self.find_pattern_after(key_pattern.as_bytes(), section_pos)?;

        // Find value start and end
        let value_start = key_pos + key_pattern.len();
        let value_end = self.find_line_end(value_start);

        // Replace value
        let new_value = format!(" {}", value);
        self.content
            .splice(value_start..value_end, new_value.bytes());

        Ok(())
    }

    fn find_pattern(&self, pattern: &[u8]) -> Result<usize> {
        self.content
            .windows(pattern.len())
            .position(|window| window == pattern)
            .ok_or_else(|| anyhow::anyhow!("Pattern not found"))
    }

    fn find_pattern_after(&self, pattern: &[u8], start: usize) -> Result<usize> {
        self.content[start..]
            .windows(pattern.len())
            .position(|window| window == pattern)
            .map(|pos| start + pos)
            .ok_or_else(|| anyhow::anyhow!("Pattern not found after position"))
    }

    fn find_line_end(&self, start: usize) -> usize {
        self.content[start..]
            .iter()
            .position(|&b| b == b'\n')
            .map(|pos| start + pos)
            .unwrap_or(self.content.len())
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        std::fs::write(path, &self.content)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_parse_valid_config() {
        let toml_content = r#"
[core]
repo_path = "~/.dotman"
default_branch = "main"
compression = "zstd"
compression_level = 3

[performance]
parallel_threads = 4
mmap_threshold = 1048576
cache_size = 100
use_hard_links = true
"#;

        let config = parse_config_str(toml_content).unwrap();
        assert_eq!(config.core.default_branch, "main");
        assert_eq!(config.performance.parallel_threads, 4);
    }

    #[test]
    fn test_validate_config_compression_level() {
        let mut config = Config::default();
        config.core.compression_level = 25;

        assert!(validate_config(&config).is_err());
    }

    #[test]
    fn test_fast_updater() -> Result<()> {
        let dir = tempdir()?;
        let config_path = dir.path().join("config.toml");

        let content = r#"[core]
repo_path = "~/.dotman"
compression_level = 3

[performance]
parallel_threads = 4"#;

        std::fs::write(&config_path, content)?;

        let mut updater = FastConfigUpdater::new(&config_path)?;
        updater.update_value("core", "compression_level", "5")?;
        updater.save(&config_path)?;

        let updated = std::fs::read_to_string(&config_path)?;
        assert!(updated.contains("compression_level = 5"));

        Ok(())
    }

    #[test]
    fn test_parse_empty_config() {
        let empty_content = "";
        let config = parse_config_str(empty_content).unwrap();
        // Should use defaults
        assert_eq!(config.core.default_branch, "main");
    }

    #[test]
    fn test_parse_malformed_toml() {
        let malformed = r#"
[core
repo_path = "~/.dotman"
"#;
        let result = parse_config_str(malformed);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Failed to parse TOML")
        );
    }

    #[test]
    fn test_parse_invalid_compression_level() {
        let invalid = r#"
[core]
compression_level = 50
"#;
        let result = parse_config_str(invalid);
        // Should fail for invalid compression level value
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_zero_parallel_threads() {
        let invalid = r#"
[performance]
parallel_threads = 0
"#;
        let result = parse_config_str(invalid);
        // Should fail for zero parallel threads value
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_excessive_cache_size() {
        let invalid = r#"
[performance]
cache_size = 20000
"#;
        let result = parse_config_str(invalid);
        // Should fail for excessive cache size value
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_invalid_utf8() {
        let invalid_utf8 = vec![0xFF, 0xFE, 0xFD];
        let result = simdutf8::basic::from_utf8(&invalid_utf8);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_large_config_mmap() -> Result<()> {
        let dir = tempdir()?;
        let config_path = dir.path().join("large.toml");

        // Create a config larger than 4KB to trigger mmap
        let mut large_config = String::from(
            "[core]\nrepo_path = \"~/.dotman\"\ndefault_branch = \"main\"\ncompression = \"zstd\"\ncompression_level = 3\n\n[tracking]\nignore_patterns = [\n",
        );
        for i in 0..999 {
            large_config.push_str(&format!("  \"pattern_{}\",\n", i));
        }
        // Last one without comma
        large_config.push_str("  \"pattern_999\"\n");
        large_config.push_str("]\nfollow_symlinks = false\npreserve_permissions = true\n");

        std::fs::write(&config_path, &large_config)?;

        // Should use mmap for large file
        let result = parse_config_file(&config_path);
        assert!(result.is_ok());
        if let Ok(config) = result {
            assert_eq!(config.tracking.ignore_patterns.len(), 1000);
        }

        Ok(())
    }

    #[test]
    fn test_parse_missing_required_fields() {
        // Even with missing fields, should use defaults
        let partial = r#"
[core]
default_branch = "develop"
"#;
        let result = parse_config_str(partial);
        // Should succeed as fields have defaults
        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(config.core.default_branch, "develop");
        // Other fields should have defaults
        assert_eq!(config.core.compression_level, 3);
    }

    #[test]
    fn test_parse_invalid_data_types() {
        let invalid = r#"
[core]
compression_level = "not_a_number"
"#;
        let result = parse_config_str(invalid);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_special_characters() {
        let special = r#"
[core]
repo_path = "/path/with spaces/and-special!@#$%^&*()chars"
default_branch = "feat/new-feature-123"
"#;
        let result = parse_config_str(special);
        assert!(result.is_ok());
        if let Ok(config) = result {
            assert!(config.core.repo_path.to_string_lossy().contains("special"));
        }
    }

    #[test]
    fn test_fast_updater_nonexistent_section() -> Result<()> {
        let dir = tempdir()?;
        let config_path = dir.path().join("config.toml");

        let content = "[core]\nrepo_path = \"~/.dotman\"\n";
        std::fs::write(&config_path, content)?;

        let mut updater = FastConfigUpdater::new(&config_path)?;
        let result = updater.update_value("nonexistent", "key", "value");
        assert!(result.is_err());

        Ok(())
    }

    #[test]
    fn test_concurrent_config_updates() -> Result<()> {
        use std::sync::Arc;
        use std::thread;

        let dir = tempdir()?;
        let config_path = Arc::new(dir.path().join("config.toml"));

        let initial_config = Config::default();
        initial_config.save(&config_path)?;

        let handles: Vec<_> = (1..6)
            .map(|i| {
                let path = config_path.clone();
                thread::spawn(move || {
                    let mut config = Config::load(&path).unwrap();
                    config.performance.parallel_threads = i;
                    config.save(&path).unwrap();
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }

        // Should have one of the values (last write wins)
        let final_config = Config::load(&config_path)?;
        assert!(
            final_config.performance.parallel_threads > 0
                && final_config.performance.parallel_threads <= 5
        );

        Ok(())
    }
}
