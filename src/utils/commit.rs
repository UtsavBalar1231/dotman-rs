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
/// Creates a deterministic 32-character hex string based on commit content.
///
/// Parent order is significant: first parent is the "mainline" branch (what you were on),
/// second parent is what you merged in. This mirrors Git's semantics for `--first-parent`.
#[must_use]
pub fn generate_commit_id(
    tree_hash: &str,
    parents: &[&str],
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

    // Add parents in order (first parent = mainline, second = merged branch)
    for parent_id in parents {
        commit_content.push_str("parent ");
        commit_content.push_str(parent_id);
        commit_content.push('\n');
    }

    commit_content.push_str("author ");
    commit_content.push_str(author);
    commit_content.push(' ');
    #[allow(clippy::expect_used)] // Writing to String never fails
    write!(&mut commit_content, "{timestamp}.{nanos:09}").expect("Writing to string cannot fail");
    commit_content.push('\n');

    // Add message
    commit_content.push_str("message ");
    commit_content.push_str(message);

    // Generate hash of the complete commit content
    hash_bytes(commit_content.as_bytes())
}
