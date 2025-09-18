use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// Represents a single entry in the reflog
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReflogEntry {
    /// Unix timestamp when the operation occurred
    pub timestamp: i64,
    /// Previous HEAD value (commit hash or symbolic ref)
    pub old_value: String,
    /// New HEAD value (commit hash or symbolic ref)
    pub new_value: String,
    /// Type of operation that caused the HEAD change
    pub operation: String,
    /// Descriptive message about the operation
    pub message: String,
}

impl ReflogEntry {
    /// Create a new reflog entry with current timestamp
    #[must_use]
    pub fn new(old_value: String, new_value: String, operation: String, message: String) -> Self {
        let timestamp = i64::try_from(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        )
        .unwrap_or(i64::MAX);

        Self {
            timestamp,
            old_value,
            new_value,
            operation,
            message,
        }
    }

    /// Format entry as a single line for storage
    #[must_use]
    pub fn to_line(&self) -> String {
        format!(
            "{} {} {} {}: {}",
            self.timestamp, self.old_value, self.new_value, self.operation, self.message
        )
    }

    /// Parse a line from storage back into a `ReflogEntry`
    ///
    /// # Errors
    ///
    /// Returns an error if the line format is invalid or cannot be parsed
    pub fn from_line(line: &str) -> Result<Self> {
        let parts: Vec<&str> = line.splitn(5, ' ').collect();
        if parts.len() < 5 {
            return Err(anyhow::anyhow!("Invalid reflog entry format"));
        }

        let timestamp = parts[0].parse::<i64>()?;
        let old_value = parts[1].to_string();
        let new_value = parts[2].to_string();

        // The fourth part is "operation:" and the fifth part is the message
        let operation = parts[3].trim_end_matches(':').to_string();
        let message = parts[4].trim().to_string();

        Ok(Self {
            timestamp,
            old_value,
            new_value,
            operation,
            message,
        })
    }

    /// Get short commit hash (first 8 characters) for display
    /// Only truncates actual commit hashes, not symbolic references
    #[must_use]
    pub fn short_hash(&self) -> &str {
        // Don't truncate symbolic references (e.g., "ref: refs/heads/main")
        if self.new_value.starts_with("ref:") {
            &self.new_value
        } else if self.new_value.len() >= 8 && self.new_value.chars().all(|c| c.is_ascii_hexdigit())
        {
            // Only truncate hexadecimal commit hashes
            &self.new_value[..8]
        } else {
            &self.new_value
        }
    }
}

/// Manages reflog operations for HEAD
pub struct ReflogManager {
    repo_path: PathBuf,
    logs_dir: PathBuf,
    head_log_path: PathBuf,
}

impl ReflogManager {
    /// Create a new `ReflogManager` for the given repository
    #[must_use]
    pub fn new(repo_path: PathBuf) -> Self {
        let logs_dir = repo_path.join("logs");
        let head_log_path = logs_dir.join("HEAD");

        Self {
            repo_path,
            logs_dir,
            head_log_path,
        }
    }

    /// Initialize the logs directory structure
    ///
    /// # Errors
    ///
    /// Returns an error if directory creation fails
    pub fn init(&self) -> Result<()> {
        fs::create_dir_all(&self.logs_dir)?;
        Ok(())
    }

    /// Add a new entry to the HEAD reflog
    ///
    /// # Errors
    ///
    /// Returns an error if the reflog file cannot be written
    pub fn log_head_update(
        &self,
        old_value: &str,
        new_value: &str,
        operation: &str,
        message: &str,
    ) -> Result<()> {
        // Ensure logs directory exists
        self.init()?;

        let entry = ReflogEntry::new(
            old_value.to_string(),
            new_value.to_string(),
            operation.to_string(),
            message.to_string(),
        );

        // Append to HEAD log file
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.head_log_path)?;

        writeln!(file, "{}", entry.to_line())?;
        file.flush()?;

        Ok(())
    }

    /// Read all entries from the HEAD reflog
    ///
    /// # Errors
    ///
    /// Returns an error if the reflog file cannot be read
    pub fn read_head_log(&self) -> Result<Vec<ReflogEntry>> {
        if !self.head_log_path.exists() {
            return Ok(Vec::new());
        }

        let file = fs::File::open(&self.head_log_path)?;
        let reader = BufReader::new(file);

        let mut entries = Vec::new();
        for line in reader.lines() {
            let line = line?;
            if !line.trim().is_empty() {
                match ReflogEntry::from_line(&line) {
                    Ok(entry) => entries.push(entry),
                    Err(_) => {
                        // Skip malformed lines but continue processing
                        eprintln!("Warning: skipping malformed reflog entry: {line}");
                    }
                }
            }
        }

        Ok(entries)
    }

    /// Get the current HEAD value for reflog operations
    ///
    /// # Errors
    ///
    /// Returns an error if HEAD file cannot be read or resolved
    pub fn get_current_head(&self) -> Result<String> {
        let head_path = self.repo_path.join("HEAD");
        if !head_path.exists() {
            return Ok("0".repeat(40)); // Initial empty repository state
        }

        let head_content = fs::read_to_string(&head_path)?.trim().to_string();

        // If HEAD points to a branch, resolve to the actual commit
        if let Some(branch_name) = head_content.strip_prefix("ref: refs/heads/") {
            let branch_path = self.repo_path.join(format!("refs/heads/{branch_name}"));
            if branch_path.exists() {
                return Ok(fs::read_to_string(&branch_path)?.trim().to_string());
            }
            // Branch doesn't exist yet, return zeros
            return Ok("0".repeat(40));
        }

        // HEAD points directly to a commit (detached)
        Ok(head_content)
    }
}
