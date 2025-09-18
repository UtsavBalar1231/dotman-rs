use super::Config;
use anyhow::{Context, Result};
use memmap2::MmapOptions;
use std::fs::File;
use std::path::Path;

// Fast TOML parser optimized for our config structure
/// Parse a configuration file from disk
///
/// # Errors
///
/// Returns an error if:
/// - File cannot be read
/// - File contains invalid UTF-8
/// - TOML parsing fails
pub fn parse_config_file(path: &Path) -> Result<Config> {
    // For small files, use regular reading
    let metadata = std::fs::metadata(path)?;

    if metadata.len() < 4096 {
        // Small file - read normally
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;
        parse_config_str(&content)
    } else {
        // Large file - use memory mapping
        let file = File::open(path)?;
        let mmap = unsafe { MmapOptions::new().map(&file)? };

        // Validate UTF-8 using SIMD
        let content =
            simdutf8::basic::from_utf8(&mmap).with_context(|| "Invalid UTF-8 in config file")?;

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
        return Err(anyhow::anyhow!(
            "Compression level must be between 1 and 22"
        ));
    }

    // Validate thread count
    if config.performance.parallel_threads == 0 {
        return Err(anyhow::anyhow!("Parallel threads must be at least 1"));
    }

    Ok(())
}

// Fast key-value parser for simple config updates
pub struct FastConfigUpdater {
    content: Vec<u8>,
}

impl FastConfigUpdater {
    /// Create a new `FastConfigUpdater` from a config file
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read
    pub fn new(path: &Path) -> Result<Self> {
        let content = std::fs::read(path)?;
        Ok(Self { content })
    }

    /// Update a configuration value
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Section cannot be found
    /// - Key cannot be found within section
    pub fn update_value(&mut self, section: &str, key: &str, value: &str) -> Result<()> {
        // Use SIMD to find the section and key quickly
        let section_pattern = format!("[{section}]");
        let key_pattern = format!("{key} =");

        // Find section start
        let section_pos = self.find_pattern(section_pattern.as_bytes())?;

        // Find key within section
        let key_pos = self.find_pattern_after(key_pattern.as_bytes(), section_pos)?;

        // Find value start and end
        let value_start = key_pos + key_pattern.len();
        let value_end = self.find_line_end(value_start);

        // Replace value
        let new_value = format!(" {value}");
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
            .map_or(self.content.len(), |pos| start + pos)
    }

    /// Save the configuration back to disk
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be written
    pub fn save(&self, path: &Path) -> Result<()> {
        std::fs::write(path, &self.content)?;
        Ok(())
    }
}
