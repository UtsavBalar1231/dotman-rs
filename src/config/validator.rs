use anyhow::Result;
use colored::Colorize;
use std::collections::HashSet;
use std::path::Path;

/// Tracks which configuration fields are actually used at runtime
pub struct ConfigValidator {
    /// Set of valid configuration fields that are recognized by dotman
    known_fields: HashSet<String>,
    /// Set of deprecated configuration fields that should trigger warnings
    deprecated_fields: HashSet<String>,
}

impl ConfigValidator {
    /// Create a new validator with known configuration fields
    #[must_use]
    pub fn new() -> Self {
        let mut known_fields = HashSet::new();
        let mut deprecated_fields = HashSet::new();

        // Core fields
        known_fields.insert("core.repo_path".to_string());
        known_fields.insert("core.compression".to_string());
        known_fields.insert("core.compression_level".to_string());
        known_fields.insert("core.pager".to_string());

        // Deprecated core fields
        deprecated_fields.insert("core.default_branch".to_string());

        // User fields
        known_fields.insert("user.name".to_string());
        known_fields.insert("user.email".to_string());

        // Performance fields
        known_fields.insert("performance.parallel_threads".to_string());
        known_fields.insert("performance.mmap_threshold".to_string());
        known_fields.insert("performance.use_hard_links".to_string());

        // Tracking fields
        known_fields.insert("tracking.ignore_patterns".to_string());
        known_fields.insert("tracking.follow_symlinks".to_string());
        known_fields.insert("tracking.preserve_permissions".to_string());

        // Branch fields
        deprecated_fields.insert("branches.current".to_string());
        // Dynamic branch tracking fields are handled separately

        Self {
            known_fields,
            deprecated_fields,
        }
    }

    /// Validate a loaded configuration file and warn about issues
    ///
    /// # Errors
    ///
    /// Returns an error if the config file cannot be read or parsed
    pub fn validate_config_file(&self, config_path: &Path) -> Result<()> {
        if !config_path.exists() {
            return Ok(());
        }

        let content = std::fs::read_to_string(config_path)?;
        let parsed: toml::Value = toml::from_str(&content)?;

        let mut warnings = Vec::new();
        let mut unknown_fields = Vec::new();
        let mut deprecated_used = Vec::new();

        self.check_table(&parsed, "", &mut unknown_fields, &mut deprecated_used);

        // Collect warnings
        for field in &unknown_fields {
            warnings.push(format!("Unknown configuration field: {}", field.yellow()));
        }

        for field in &deprecated_used {
            let suggestion = match field.as_str() {
                "core.default_branch" => {
                    "This field is deprecated and has no effect. The default branch is always 'main'."
                }
                "branches.current" => {
                    "This field is deprecated and has no effect. Current branch is tracked in HEAD file."
                }
                _ => "This field is deprecated and will be removed in a future version.",
            };
            warnings.push(format!(
                "Deprecated field '{}': {}",
                field.yellow(),
                suggestion.dimmed()
            ));
        }

        // Print warnings if any
        if !warnings.is_empty() {
            eprintln!("{}", "Configuration warnings:".yellow().bold());
            for warning in warnings {
                eprintln!("  {warning}");
            }
            eprintln!();
        }

        Ok(())
    }

    /// Recursively checks a TOML table for unknown and deprecated fields
    ///
    /// This method traverses the configuration tree and validates each field against
    /// the known and deprecated field sets. It handles special cases for dynamic
    /// sections like remotes and branch tracking configurations.
    ///
    /// # Arguments
    ///
    /// * `table` - The TOML value to validate (expected to be a table)
    /// * `prefix` - The current path prefix (e.g., "core", "performance.parallel")
    /// * `unknown` - Vector to collect unknown field paths
    /// * `deprecated` - Vector to collect deprecated field paths
    fn check_table(
        &self,
        table: &toml::Value,
        prefix: &str,
        unknown: &mut Vec<String>,
        deprecated: &mut Vec<String>,
    ) {
        if let toml::Value::Table(map) = table {
            for (key, value) in map {
                let full_key = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{prefix}.{key}")
                };

                // Special handling for dynamic sections
                if full_key.starts_with("remotes.") {
                    // Remote configurations are dynamic
                    Self::check_remote_config(value, &full_key, unknown);
                    continue;
                }

                if full_key.starts_with("branches.tracking.") {
                    // Branch tracking configurations are dynamic
                    Self::check_branch_tracking(value, &full_key, unknown);
                    continue;
                }

                // Check if this is a known field
                if self.deprecated_fields.contains(&full_key) {
                    deprecated.push(full_key.clone());
                }

                if !self.known_fields.contains(&full_key)
                    && !full_key.starts_with("branches.tracking")
                    && !full_key.starts_with("remotes")
                {
                    // For nested tables, check recursively
                    if let toml::Value::Table(_) = value {
                        self.check_table(value, &full_key, unknown, deprecated);
                    } else if !matches!(value, toml::Value::Array(_)) {
                        // Unknown leaf field
                        unknown.push(full_key);
                    }
                } else if let toml::Value::Table(_) = value {
                    // Known section, check its contents
                    self.check_table(value, &full_key, unknown, deprecated);
                }
            }
        }
    }

    /// Validates remote configuration fields
    ///
    /// Checks that fields under `remotes.<name>` are valid remote configuration
    /// options. Valid fields are `remote_type` and `url`.
    ///
    /// # Arguments
    ///
    /// * `value` - The TOML value representing the remote configuration
    /// * `prefix` - The full path prefix for the remote (e.g., "remotes.origin")
    /// * `unknown` - Vector to collect unknown field paths
    fn check_remote_config(value: &toml::Value, prefix: &str, unknown: &mut Vec<String>) {
        if let toml::Value::Table(map) = value {
            for (key, _) in map {
                let full_key = format!("{prefix}.{key}");
                // Check if it's a valid remote field
                if !key.eq("remote_type") && !key.eq("url") {
                    unknown.push(full_key);
                }
            }
        }
    }

    /// Validates branch tracking configuration fields
    ///
    /// Checks that fields under `branches.tracking.<name>` are valid branch tracking
    /// options. Valid fields are `remote` and `branch`.
    ///
    /// # Arguments
    ///
    /// * `value` - The TOML value representing the branch tracking configuration
    /// * `prefix` - The full path prefix for the branch (e.g., "branches.tracking.main")
    /// * `unknown` - Vector to collect unknown field paths
    fn check_branch_tracking(value: &toml::Value, prefix: &str, unknown: &mut Vec<String>) {
        if let toml::Value::Table(map) = value {
            for (key, _) in map {
                let full_key = format!("{prefix}.{key}");
                // Check if it's a valid branch tracking field
                if !key.eq("remote") && !key.eq("branch") {
                    unknown.push(full_key);
                }
            }
        }
    }

    /// Check for unused configuration options that have no effect
    pub const fn warn_unused_options(_config: &crate::config::Config) {
        // Currently no unused options to warn about
    }
}

impl Default for ConfigValidator {
    fn default() -> Self {
        Self::new()
    }
}
