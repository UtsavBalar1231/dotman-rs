use anyhow::Result;
use std::fs;
use std::path::Path;

/// Resolves a partial commit ID to a full commit ID
/// Returns an error if no match found or multiple matches exist
pub fn resolve_partial_commit_id(repo_path: &Path, partial_id: &str) -> Result<String> {
    // If it's "HEAD", return as-is for special handling
    if partial_id == "HEAD" {
        return Ok(partial_id.to_string());
    }

    let commits_dir = repo_path.join("commits");

    if !commits_dir.exists() {
        anyhow::bail!("No commits found");
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
        0 => anyhow::bail!("No commit found matching: {}", partial_id),
        1 => Ok(matches[0].clone()),
        _ => {
            // Show the ambiguous matches
            let display_matches: Vec<String> = matches
                .iter()
                .map(|m| {
                    if m.len() > 8 {
                        format!("  {}", &m[..8])
                    } else {
                        format!("  {}", m)
                    }
                })
                .collect();
            anyhow::bail!(
                "Ambiguous commit ID '{}' matches multiple commits:\n{}",
                partial_id,
                display_matches.join("\n")
            )
        }
    }
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

        // Create test commit files
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
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("Ambiguous"));

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
}
