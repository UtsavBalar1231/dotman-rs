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
    pub fn new(old_value: String, new_value: String, operation: String, message: String) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        Self {
            timestamp,
            old_value,
            new_value,
            operation,
            message,
        }
    }

    /// Format entry as a single line for storage
    pub fn to_line(&self) -> String {
        format!(
            "{} {} {} {}: {}",
            self.timestamp, self.old_value, self.new_value, self.operation, self.message
        )
    }

    /// Parse a line from storage back into a ReflogEntry
    pub fn from_line(line: &str) -> Result<Self> {
        let parts: Vec<&str> = line.splitn(5, ' ').collect();
        if parts.len() < 5 {
            anyhow::bail!("Invalid reflog entry format");
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
    pub fn short_hash(&self) -> &str {
        if self.new_value.len() >= 8 {
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
    /// Create a new ReflogManager for the given repository
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
    pub fn init(&self) -> Result<()> {
        fs::create_dir_all(&self.logs_dir)?;
        Ok(())
    }

    /// Add a new entry to the HEAD reflog
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
                        eprintln!("Warning: skipping malformed reflog entry: {}", line);
                    }
                }
            }
        }

        Ok(entries)
    }

    /// Get the current HEAD value for reflog operations
    pub fn get_current_head(&self) -> Result<String> {
        let head_path = self.repo_path.join("HEAD");
        if !head_path.exists() {
            return Ok("0".repeat(40)); // Initial empty repository state
        }

        let head_content = fs::read_to_string(&head_path)?.trim().to_string();

        // If HEAD points to a branch, resolve to the actual commit
        if let Some(branch_name) = head_content.strip_prefix("ref: refs/heads/") {
            let branch_path = self.repo_path.join(format!("refs/heads/{}", branch_name));
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup_test_manager() -> Result<(TempDir, ReflogManager)> {
        let temp_dir = TempDir::new()?;
        let repo_path = temp_dir.path().join(".dotman");
        fs::create_dir_all(&repo_path)?;
        let manager = ReflogManager::new(repo_path);
        Ok((temp_dir, manager))
    }

    #[test]
    fn test_reflog_entry_creation() {
        let entry = ReflogEntry::new(
            "abc123".to_string(),
            "def456".to_string(),
            "commit".to_string(),
            "Initial commit".to_string(),
        );

        assert_eq!(entry.old_value, "abc123");
        assert_eq!(entry.new_value, "def456");
        assert_eq!(entry.operation, "commit");
        assert_eq!(entry.message, "Initial commit");
        assert!(entry.timestamp > 0);
    }

    #[test]
    fn test_reflog_entry_serialization() -> Result<()> {
        let entry = ReflogEntry {
            timestamp: 1640995200,
            old_value: "abc123".to_string(),
            new_value: "def456".to_string(),
            operation: "commit".to_string(),
            message: "Initial commit".to_string(),
        };

        let line = entry.to_line();
        let parsed = ReflogEntry::from_line(&line)?;

        assert_eq!(parsed.timestamp, entry.timestamp);
        assert_eq!(parsed.old_value, entry.old_value);
        assert_eq!(parsed.new_value, entry.new_value);
        assert_eq!(parsed.operation, entry.operation);
        assert_eq!(parsed.message, entry.message);

        Ok(())
    }

    #[test]
    fn test_short_hash() {
        let entry = ReflogEntry::new(
            "old".to_string(),
            "0123456789abcdef".to_string(),
            "commit".to_string(),
            "Test".to_string(),
        );

        assert_eq!(entry.short_hash(), "01234567");

        let short_entry = ReflogEntry::new(
            "old".to_string(),
            "abc".to_string(),
            "commit".to_string(),
            "Test".to_string(),
        );

        assert_eq!(short_entry.short_hash(), "abc");
    }

    #[test]
    fn test_reflog_manager_init() -> Result<()> {
        let (_temp, manager) = setup_test_manager()?;

        manager.init()?;
        assert!(manager.logs_dir.exists());

        Ok(())
    }

    #[test]
    fn test_log_head_update() -> Result<()> {
        let (_temp, manager) = setup_test_manager()?;

        manager.log_head_update("abc123", "def456", "commit", "Initial commit")?;

        assert!(manager.head_log_path.exists());

        let entries = manager.read_head_log()?;
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].old_value, "abc123");
        assert_eq!(entries[0].new_value, "def456");
        assert_eq!(entries[0].operation, "commit");
        assert_eq!(entries[0].message, "Initial commit");

        Ok(())
    }

    #[test]
    fn test_read_empty_log() -> Result<()> {
        let (_temp, manager) = setup_test_manager()?;

        let entries = manager.read_head_log()?;
        assert!(entries.is_empty());

        Ok(())
    }

    #[test]
    fn test_multiple_entries() -> Result<()> {
        let (_temp, manager) = setup_test_manager()?;

        // Log multiple updates
        manager.log_head_update("000", "abc", "commit", "First commit")?;
        manager.log_head_update("abc", "def", "commit", "Second commit")?;
        manager.log_head_update("def", "ghi", "checkout", "Switch branch")?;

        let entries = manager.read_head_log()?;
        assert_eq!(entries.len(), 3);

        // Entries should be in the order they were added
        assert_eq!(entries[0].message, "First commit");
        assert_eq!(entries[1].message, "Second commit");
        assert_eq!(entries[2].message, "Switch branch");

        Ok(())
    }

    #[test]
    fn test_get_current_head_empty_repo() -> Result<()> {
        let (_temp, manager) = setup_test_manager()?;

        let head = manager.get_current_head()?;
        assert_eq!(head, "0".repeat(40));

        Ok(())
    }

    #[test]
    fn test_get_current_head_detached() -> Result<()> {
        let (_temp, manager) = setup_test_manager()?;

        // Create HEAD pointing to commit directly
        fs::write(manager.repo_path.join("HEAD"), "abc123def456")?;

        let head = manager.get_current_head()?;
        assert_eq!(head, "abc123def456");

        Ok(())
    }

    #[test]
    fn test_get_current_head_branch() -> Result<()> {
        let (_temp, manager) = setup_test_manager()?;

        // Create refs structure
        fs::create_dir_all(manager.repo_path.join("refs/heads"))?;
        fs::write(manager.repo_path.join("refs/heads/main"), "commit123")?;
        fs::write(manager.repo_path.join("HEAD"), "ref: refs/heads/main")?;

        let head = manager.get_current_head()?;
        assert_eq!(head, "commit123");

        Ok(())
    }
}
