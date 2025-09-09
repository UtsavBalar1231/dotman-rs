use colored::Colorize;
use std::fmt;

/// Formats a commit ID for display (shows first 8 characters)
#[must_use]
pub fn format_commit_id(commit_id: &str) -> &str {
    if commit_id.len() >= 8 {
        &commit_id[..8]
    } else {
        commit_id
    }
}

/// Formats a branch name with optional current branch indicator
#[must_use]
pub fn format_branch_name(name: &str, is_current: bool) -> String {
    if is_current {
        format!("* {}", name.green().bold())
    } else {
        format!("  {name}")
    }
}

/// Represents different file statuses for formatting
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileStatus {
    Added,
    Modified,
    Deleted,
    Untracked,
    Staged,
    Conflict,
}

impl FileStatus {
    /// Returns the status character used in short format
    #[must_use]
    pub const fn short_char(&self) -> char {
        match self {
            Self::Added => 'A',
            Self::Modified => 'M',
            Self::Deleted => 'D',
            Self::Untracked => '?',
            Self::Staged => 'S',
            Self::Conflict => 'C',
        }
    }

    /// Returns the colored status character
    #[must_use]
    pub fn colored_char(&self) -> String {
        match self {
            Self::Added => "A".green().to_string(),
            Self::Modified => "M".yellow().to_string(),
            Self::Deleted => "D".red().to_string(),
            Self::Untracked => "?".bright_black().to_string(),
            Self::Staged => "S".blue().to_string(),
            Self::Conflict => "C".red().bold().to_string(),
        }
    }

    /// Returns the full status name
    #[must_use]
    pub const fn name(&self) -> &str {
        match self {
            Self::Added => "added",
            Self::Modified => "modified",
            Self::Deleted => "deleted",
            Self::Untracked => "untracked",
            Self::Staged => "staged",
            Self::Conflict => "conflict",
        }
    }
}

impl fmt::Display for FileStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// Formats a file path with its status
#[must_use]
pub fn format_file_status(status: FileStatus, path: &str) -> String {
    format!("{} {path}", status.colored_char())
}

/// Formats a file path with its status for short output
#[must_use]
pub fn format_file_status_short(status: FileStatus, path: &str) -> String {
    format!("{} {path}", status.short_char())
}

/// Formats bytes into human-readable size
#[must_use]
pub fn format_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    #[allow(clippy::cast_precision_loss)]
    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{bytes} {}", UNITS[unit_index])
    } else {
        format!("{size:.2} {}", UNITS[unit_index])
    }
}

/// Formats a timestamp in a human-readable format
#[must_use]
pub fn format_timestamp(timestamp: i64) -> String {
    use chrono::{Local, TimeZone};

    let datetime = Local.timestamp_opt(timestamp, 0).single();
    datetime.map_or_else(
        || format!("Invalid timestamp: {timestamp}"),
        |dt| dt.format("%Y-%m-%d %H:%M:%S").to_string(),
    )
}

/// Formats a relative time from now
#[must_use]
pub fn format_relative_time(timestamp: i64) -> String {
    use chrono::Utc;

    let now = Utc::now().timestamp();
    let diff = now - timestamp;

    if diff < 0 {
        return "in the future".to_string();
    }

    let (value, unit) = if diff < 60 {
        (diff, "second")
    } else if diff < 3600 {
        (diff / 60, "minute")
    } else if diff < 86400 {
        (diff / 3600, "hour")
    } else if diff < 2_592_000 {
        (diff / 86400, "day")
    } else if diff < 31_536_000 {
        (diff / 2_592_000, "month")
    } else {
        (diff / 31_536_000, "year")
    };

    if value == 1 {
        format!("{value} {unit} ago")
    } else {
        format!("{value} {unit}s ago")
    }
}

/// Truncates a string to a maximum length with ellipsis
#[must_use]
pub fn truncate_with_ellipsis(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else if max_len < 3 {
        s[..max_len].to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_commit_id() {
        assert_eq!(format_commit_id("1234567890abcdef"), "12345678");
        assert_eq!(format_commit_id("12345"), "12345");
        assert_eq!(format_commit_id("123456789"), "12345678");
    }

    #[test]
    fn test_format_branch_name() {
        let current = format_branch_name("main", true);
        assert!(current.contains('*'));
        assert!(current.contains("main"));

        let other = format_branch_name("feature", false);
        assert!(!other.contains('*'));
        assert!(other.contains("feature"));
    }

    #[test]
    fn test_file_status() {
        assert_eq!(FileStatus::Added.short_char(), 'A');
        assert_eq!(FileStatus::Modified.short_char(), 'M');
        assert_eq!(FileStatus::Deleted.short_char(), 'D');
        assert_eq!(FileStatus::Untracked.short_char(), '?');

        assert_eq!(FileStatus::Added.name(), "added");
        assert_eq!(FileStatus::Modified.name(), "modified");
    }

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(1023), "1023 B");
        assert_eq!(format_size(1024), "1.00 KB");
        assert_eq!(format_size(1536), "1.50 KB");
        assert_eq!(format_size(1_048_576), "1.00 MB");
        assert_eq!(format_size(1_073_741_824), "1.00 GB");
    }

    #[test]
    fn test_truncate_with_ellipsis() {
        assert_eq!(truncate_with_ellipsis("hello world", 20), "hello world");
        assert_eq!(truncate_with_ellipsis("hello world", 8), "hello...");
        assert_eq!(truncate_with_ellipsis("hello world", 5), "he...");
        assert_eq!(truncate_with_ellipsis("hi", 2), "hi");
        assert_eq!(truncate_with_ellipsis("hello", 2), "he");
    }
}
