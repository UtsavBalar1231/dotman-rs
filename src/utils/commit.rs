use crate::storage::file_ops::hash_bytes;
use anyhow::Result;
use std::fmt::Write;
use std::fs;
use std::path::Path;

/// Resolves a partial commit ID to a full commit ID
/// Returns an error if no match found or multiple matches exist
///
/// # Errors
///
/// Returns an error if:
/// - No commits exist in the repository
/// - No commit matches the partial ID
/// - Multiple commits match the partial ID (ambiguous)
/// - Failed to read the commits directory
pub fn resolve_partial_commit_id(repo_path: &Path, partial_id: &str) -> Result<String> {
    // If it's "HEAD", return as-is for special handling
    if partial_id == "HEAD" {
        return Ok(partial_id.to_string());
    }

    let commits_dir = repo_path.join("commits");

    if !commits_dir.exists() {
        return Err(anyhow::anyhow!("No commits found"));
    }

    let mut matches = Vec::new();

    for entry in fs::read_dir(&commits_dir)? {
        let entry = entry?;
        let path = entry.path();

        if let Some(stem) = path.file_stem().and_then(|s| s.to_str())
            && stem.starts_with(partial_id)
        {
            matches.push(stem.to_string());
        }
    }

    match matches.len() {
        0 => Err(anyhow::anyhow!("No commit found matching: {partial_id}")),
        1 => Ok(matches[0].clone()),
        _ => {
            // Show the ambiguous matches
            let display_matches: Vec<String> = matches
                .iter()
                .map(|m| {
                    if m.len() > 8 {
                        format!("  {}", &m[..8])
                    } else {
                        format!("  {m}")
                    }
                })
                .collect();
            Err(anyhow::anyhow!(
                "Ambiguous commit ID '{}' matches multiple commits:\n{}",
                partial_id,
                display_matches.join("\n")
            ))
        }
    }
}

/// Generates a content-addressed commit ID using xxhash
/// Creates a deterministic 32-character hex string based on commit content
#[must_use]
pub fn generate_commit_id(
    tree_hash: &str,
    parent: Option<&str>,
    message: &str,
    author: &str,
    timestamp: i64,
    nanos: u32,
) -> String {
    let mut commit_content = String::new();

    // Add tree hash
    commit_content.push_str("tree ");
    commit_content.push_str(tree_hash);
    commit_content.push('\n');

    // Add parent if it exists
    if let Some(parent_id) = parent {
        commit_content.push_str("parent ");
        commit_content.push_str(parent_id);
        commit_content.push('\n');
    }

    commit_content.push_str("author ");
    commit_content.push_str(author);
    commit_content.push(' ');
    write!(&mut commit_content, "{timestamp}.{nanos:09}").expect("Writing to string cannot fail");
    commit_content.push('\n');

    // Add message
    commit_content.push_str("message ");
    commit_content.push_str(message);

    // Generate hash of the complete commit content
    hash_bytes(commit_content.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_resolve_head() -> Result<()> {
        let temp = tempdir()?;
        let result = resolve_partial_commit_id(temp.path(), "HEAD")?;
        assert_eq!(result, "HEAD");
        Ok(())
    }

    #[test]
    fn test_resolve_no_commits() -> Result<()> {
        let temp = tempdir()?;
        let result = resolve_partial_commit_id(temp.path(), "abc");
        assert!(result.is_err());
        Ok(())
    }

    #[test]
    fn test_resolve_unique_match() -> Result<()> {
        let temp = tempdir()?;
        let commits_dir = temp.path().join("commits");
        fs::create_dir_all(&commits_dir)?;

        fs::write(commits_dir.join("abc123def456.zst"), "")?;
        fs::write(commits_dir.join("def789ghi012.zst"), "")?;

        let result = resolve_partial_commit_id(temp.path(), "abc")?;
        assert_eq!(result, "abc123def456");

        Ok(())
    }

    #[test]
    fn test_resolve_ambiguous() -> Result<()> {
        let temp = tempdir()?;
        let commits_dir = temp.path().join("commits");
        fs::create_dir_all(&commits_dir)?;

        // Create ambiguous commits
        fs::write(commits_dir.join("abc123def456.zst"), "")?;
        fs::write(commits_dir.join("abc789ghi012.zst"), "")?;

        let result = resolve_partial_commit_id(temp.path(), "abc");
        // Should fail when partial commit ID matches multiple commits
        assert!(result.is_err());

        Ok(())
    }

    #[test]
    fn test_resolve_full_id() -> Result<()> {
        let temp = tempdir()?;
        let commits_dir = temp.path().join("commits");
        fs::create_dir_all(&commits_dir)?;

        fs::write(commits_dir.join("abc123def456.zst"), "")?;

        let result = resolve_partial_commit_id(temp.path(), "abc123def456")?;
        assert_eq!(result, "abc123def456");

        Ok(())
    }

    #[test]
    fn test_generate_commit_id_deterministic() {
        let tree_hash = "abcd1234";
        let parent = Some("parent123");
        let message = "Test commit";
        let author = "Test User";
        let timestamp = 1_234_567_890i64;
        let nanos = 123_456_789u32;

        let id1 = generate_commit_id(tree_hash, parent, message, author, timestamp, nanos);
        let id2 = generate_commit_id(tree_hash, parent, message, author, timestamp, nanos);

        assert_eq!(id1, id2);
        assert_eq!(id1.len(), 32); // 32 hex characters
        assert!(id1.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_generate_commit_id_different_inputs() {
        let tree_hash = "abcd1234";
        let parent = Some("parent123");
        let message = "Test commit";
        let author = "Test User";
        let timestamp = 1_234_567_890i64;
        let nanos = 0u32;

        let id1 = generate_commit_id(tree_hash, parent, message, author, timestamp, nanos);

        // Different message should produce different ID
        let id2 = generate_commit_id(
            tree_hash,
            parent,
            "Different message",
            author,
            timestamp,
            nanos,
        );
        assert_ne!(id1, id2);

        // Different timestamp should produce different ID
        let id3 = generate_commit_id(tree_hash, parent, message, author, timestamp + 1, nanos);
        assert_ne!(id1, id3);

        // Different nanoseconds should produce different ID
        let id4 = generate_commit_id(tree_hash, parent, message, author, timestamp, nanos + 1);
        assert_ne!(id1, id4);
    }

    #[test]
    fn test_generate_commit_id_no_parent() {
        // Test commit without parent (initial commit)
        let tree_hash = "abcd1234";
        let message = "Initial commit";
        let author = "Test User";
        let timestamp = 1_234_567_890i64;
        let nanos = 0u32;

        let id1 = generate_commit_id(tree_hash, None, message, author, timestamp, nanos);
        let id2 = generate_commit_id(
            tree_hash,
            Some("parent123"),
            message,
            author,
            timestamp,
            nanos,
        );

        assert_ne!(id1, id2); // Should be different with and without parent
        assert_eq!(id1.len(), 32);
        assert!(id1.chars().all(|c| c.is_ascii_hexdigit()));
    }
}
