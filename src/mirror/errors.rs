use std::fmt;

/// Categorized git operation errors with actionable guidance
#[derive(Debug)]
pub enum GitError {
    /// Network-related errors (DNS, connection timeout, unreachable)
    Network(String),
    /// Authentication failures (SSH keys, passwords, tokens)
    Authentication(String),
    /// Resource not found (branch, tag, commit, remote)
    NotFound(String),
    /// Merge or push conflicts (non-fast-forward, merge conflicts)
    Conflict(String),
    /// File system permission errors
    Permission(String),
    /// Invalid reference name or format
    InvalidRef(String),
    /// Unknown or uncategorized error
    Unknown(String),
}

impl GitError {
    /// Parse git command stderr to categorize the error
    ///
    /// Analyzes common git error patterns to provide better error messages
    /// and actionable guidance to users.
    #[must_use]
    pub fn from_stderr(command: &str, stderr: &str) -> Self {
        let stderr_lower = stderr.to_lowercase();

        // Network errors
        if stderr_lower.contains("could not resolve host")
            || stderr_lower.contains("connection timed out")
            || stderr_lower.contains("network is unreachable")
            || stderr_lower.contains("failed to connect")
            || stderr_lower.contains("connection refused")
        {
            return Self::Network(format!(
                "{}: Network error - {}",
                command,
                extract_meaningful_message(stderr)
            ));
        }

        // Authentication errors
        if stderr_lower.contains("authentication failed")
            || stderr_lower.contains("permission denied")
            || stderr_lower.contains("publickey")
            || stderr_lower.contains("access denied")
            || stderr_lower.contains("invalid credentials")
            || stderr_lower.contains("could not read username")
        {
            return Self::Authentication(format!(
                "{}: Authentication failed - {}",
                command,
                extract_meaningful_message(stderr)
            ));
        }

        // Not found errors
        if stderr_lower.contains("does not exist")
            || stderr_lower.contains("not found")
            || stderr_lower.contains("couldn't find remote ref")
            || stderr_lower.contains("remote not found")
            || stderr_lower.contains("no such")
        {
            return Self::NotFound(format!(
                "{}: Resource not found - {}",
                command,
                extract_meaningful_message(stderr)
            ));
        }

        // Conflict errors
        if stderr_lower.contains("non-fast-forward")
            || stderr_lower.contains("rejected")
            || stderr_lower.contains("conflict")
            || stderr_lower.contains("failed to push some refs")
            || stderr_lower.contains("merge conflict")
        {
            return Self::Conflict(format!(
                "{}: Conflict detected - {}",
                command,
                extract_meaningful_message(stderr)
            ));
        }

        // Permission errors
        if stderr_lower.contains("permission denied (os)")
            || stderr_lower.contains("unable to create")
            || stderr_lower.contains("read-only")
            || stderr_lower.contains("cannot open")
        {
            return Self::Permission(format!(
                "{}: Permission error - {}",
                command,
                extract_meaningful_message(stderr)
            ));
        }

        // Invalid ref errors
        if stderr_lower.contains("invalid ref")
            || stderr_lower.contains("malformed")
            || stderr_lower.contains("bad revision")
            || stderr_lower.contains("ambiguous argument")
        {
            return Self::InvalidRef(format!(
                "{}: Invalid reference - {}",
                command,
                extract_meaningful_message(stderr)
            ));
        }

        // Unknown error
        Self::Unknown(format!(
            "{}: {}",
            command,
            extract_meaningful_message(stderr)
        ))
    }

    /// Get a user-friendly error message with actionable guidance
    #[must_use]
    pub fn user_message(&self) -> String {
        match self {
            Self::Network(msg) => format!(
                "{msg}\n\nSuggestions:\n\
                 - Check your internet connection\n\
                 - Verify the remote URL is correct\n\
                 - Try again in a few moments\n\
                 - Check if a firewall or proxy is blocking the connection"
            ),
            Self::Authentication(msg) => format!(
                "{msg}\n\nSuggestions:\n\
                 - Verify your SSH key is configured (ssh-add -l)\n\
                 - Check if your credentials are correct\n\
                 - Ensure your token/password hasn't expired\n\
                 - For HTTPS, you may need to set up a credential helper"
            ),
            Self::NotFound(msg) => format!(
                "{msg}\n\nSuggestions:\n\
                 - Verify the branch/tag/commit exists\n\
                 - Check if the remote URL is correct\n\
                 - Ensure you've fetched the latest changes (dot fetch)"
            ),
            Self::Conflict(msg) => format!(
                "{msg}\n\nSuggestions:\n\
                 - Pull the latest changes first (dot pull)\n\
                 - Resolve any merge conflicts\n\
                 - Use --force if you want to overwrite (be careful!)\n\
                 - Use --force-with-lease for safer force push"
            ),
            Self::Permission(msg) => format!(
                "{msg}\n\nSuggestions:\n\
                 - Check file and directory permissions\n\
                 - Ensure you have write access to the repository\n\
                 - Check if another process has the file locked"
            ),
            Self::InvalidRef(msg) => format!(
                "{msg}\n\nSuggestions:\n\
                 - Check the branch/tag name for invalid characters\n\
                 - Ensure the ref format is correct\n\
                 - Use 'dot branch' or 'dot tag' to see valid refs"
            ),
            Self::Unknown(msg) => format!(
                "{msg}\n\nThis is an unexpected error. Please check the message above for details."
            ),
        }
    }

    /// Check if this error type is transient and might succeed on retry
    #[must_use]
    pub const fn should_retry(&self) -> bool {
        matches!(self, Self::Network(_))
    }

    /// Get a short description of the error type
    #[must_use]
    pub const fn error_type(&self) -> &'static str {
        match self {
            Self::Network(_) => "Network Error",
            Self::Authentication(_) => "Authentication Error",
            Self::NotFound(_) => "Not Found",
            Self::Conflict(_) => "Conflict",
            Self::Permission(_) => "Permission Denied",
            Self::InvalidRef(_) => "Invalid Reference",
            Self::Unknown(_) => "Unknown Error",
        }
    }
}

impl fmt::Display for GitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.user_message())
    }
}

impl std::error::Error for GitError {}

/// Extract the most meaningful part of the error message
///
/// Removes noise and focuses on the actual error description
fn extract_meaningful_message(stderr: &str) -> String {
    // Take first 3 non-empty lines (usually contains the key info)
    let lines: Vec<&str> = stderr
        .lines()
        .filter(|l| !l.trim().is_empty())
        .take(3)
        .collect();

    if lines.is_empty() {
        return "No error details available".to_string();
    }

    lines.join(" | ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_error_detection() {
        let stderr = "fatal: Could not resolve host: github.com";
        let error = GitError::from_stderr("git fetch", stderr);
        assert!(matches!(error, GitError::Network(_)));
        assert!(error.should_retry());
    }

    #[test]
    fn test_auth_error_detection() {
        let stderr = "fatal: Authentication failed for 'https://github.com/user/repo.git'";
        let error = GitError::from_stderr("git push", stderr);
        assert!(matches!(error, GitError::Authentication(_)));
        assert!(!error.should_retry());
    }

    #[test]
    fn test_conflict_error_detection() {
        let stderr = "error: failed to push some refs\nhint: Updates were rejected because the tip of your current branch is behind";
        let error = GitError::from_stderr("git push", stderr);
        assert!(matches!(error, GitError::Conflict(_)));
    }

    #[test]
    fn test_not_found_error_detection() {
        let stderr = "fatal: Remote branch 'nonexistent' not found";
        let error = GitError::from_stderr("git fetch", stderr);
        assert!(matches!(error, GitError::NotFound(_)));
    }
}
