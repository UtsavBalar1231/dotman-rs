use crate::refs::RefManager;
use crate::storage::snapshots::SnapshotManager;
use anyhow::{Context, Result};
use std::path::PathBuf;

/// Resolves various reference formats to commit IDs
pub struct RefResolver {
    repo_path: PathBuf,
    ref_manager: RefManager,
}

impl RefResolver {
    pub fn new(repo_path: PathBuf) -> Self {
        let ref_manager = RefManager::new(repo_path.clone());
        Self {
            repo_path,
            ref_manager,
        }
    }

    /// Resolve a reference string to a commit ID
    /// Supports:
    /// - HEAD
    /// - HEAD~n (nth parent)
    /// - HEAD^ (first parent), HEAD^^ (second ancestor), HEAD^n (nth ancestor)
    /// - Branch names
    /// - Tag names (future)
    /// - Full commit IDs
    /// - Short commit IDs (prefix matching)
    /// - ref: refs/heads/branch format
    pub fn resolve(&self, reference: &str) -> Result<String> {
        // Handle ref: format (e.g., "ref: refs/heads/main")
        if let Some(branch) = reference.strip_prefix("ref: refs/heads/") {
            return self.resolve_branch(branch);
        }

        // Handle HEAD and HEAD~n
        if reference == "HEAD" {
            return self.resolve_head();
        } else if let Some(parent_spec) = reference.strip_prefix("HEAD~") {
            let parent_count = parent_spec
                .parse::<usize>()
                .with_context(|| format!("Invalid parent specification: {}", reference))?;
            return self.resolve_head_parent(parent_count);
        }
        // Handle HEAD^, HEAD^^, HEAD^^^, HEAD^n
        else if reference.starts_with("HEAD^") {
            let caret_spec = &reference[5..]; // Skip "HEAD^"
            let parent_count = self.parse_caret_notation(caret_spec, reference)?;
            return self.resolve_head_parent(parent_count);
        }

        // Try as branch name
        if self.ref_manager.branch_exists(reference) {
            return self.resolve_branch(reference);
        }

        // Try as full commit ID (must be 32 chars for our format)
        if reference.len() == 32 && reference.chars().all(|c| c.is_ascii_hexdigit()) {
            // Verify the commit exists
            let snapshot_manager = SnapshotManager::new(self.repo_path.clone(), 3);
            if snapshot_manager.snapshot_exists(reference) {
                return Ok(reference.to_string());
            }
        }

        // Try as short commit ID (prefix matching)
        if reference.len() >= 4
            && reference.chars().all(|c| c.is_ascii_hexdigit())
            && let Some(full_id) = self.find_commit_by_prefix(reference)?
        {
            return Ok(full_id);
        }

        anyhow::bail!("Cannot resolve reference: {}", reference)
    }

    /// Resolve HEAD to current commit
    fn resolve_head(&self) -> Result<String> {
        self.ref_manager
            .get_head_commit()?
            .ok_or_else(|| anyhow::anyhow!("No commits yet"))
    }

    /// Resolve HEAD~n to nth parent commit
    fn resolve_head_parent(&self, parent_count: usize) -> Result<String> {
        if parent_count == 0 {
            return self.resolve_head();
        }

        let mut current = self.resolve_head()?;
        let snapshot_manager = SnapshotManager::new(self.repo_path.clone(), 3);

        for i in 0..parent_count {
            // Handle case where current points to null SHA (initial repository state)
            if current == "0".repeat(40) || current.chars().all(|c| c == '0') {
                anyhow::bail!(
                    "Cannot go back {} commits from HEAD (only {} commits in history)",
                    parent_count,
                    i
                );
            }
            
            let snapshot = match snapshot_manager.load_snapshot(&current) {
                Ok(s) => s,
                Err(_) => anyhow::bail!(
                    "Cannot go back {} commits from HEAD (only {} commits in history)",
                    parent_count,
                    i
                ),
            };

            if let Some(parent) = snapshot.commit.parent {
                current = parent;
            } else {
                anyhow::bail!(
                    "Cannot go back {} commits from HEAD (only {} commits in history)",
                    parent_count,
                    i + 1
                );
            }
        }

        Ok(current)
    }

    /// Parse caret notation (^, ^^, ^^^, ^n) into parent count
    /// Supports:
    /// - "" (empty) -> 1 (HEAD^ means first parent)
    /// - "^" -> 2 (HEAD^^ means second ancestor)
    /// - "^^" -> 3 (HEAD^^^ means third ancestor)
    /// - "n" (number) -> n (HEAD^2 means second ancestor)
    fn parse_caret_notation(&self, caret_spec: &str, full_reference: &str) -> Result<usize> {
        if caret_spec.is_empty() {
            // HEAD^ means first parent
            return Ok(1);
        }

        // Check if it's all carets (HEAD^^, HEAD^^^, etc.)
        if caret_spec.chars().all(|c| c == '^') {
            // Each additional caret adds one to the parent count
            // HEAD^^ = 2, HEAD^^^ = 3, etc.
            return Ok(caret_spec.len() + 1);
        }

        // Check if it's a number (HEAD^2, HEAD^3, etc.)
        if let Ok(num) = caret_spec.parse::<usize>() {
            return Ok(num);
        }

        // Invalid caret notation
        anyhow::bail!("Invalid parent specification: {}", full_reference)
    }

    /// Resolve a branch name to commit ID
    fn resolve_branch(&self, branch: &str) -> Result<String> {
        self.ref_manager.get_branch_commit(branch)
    }

    /// Find a commit by prefix (short hash)
    fn find_commit_by_prefix(&self, prefix: &str) -> Result<Option<String>> {
        let commits_dir = self.repo_path.join("commits");
        if !commits_dir.exists() {
            return Ok(None);
        }

        let mut matches = Vec::new();
        for entry in std::fs::read_dir(&commits_dir)? {
            let entry = entry?;
            let name = entry.file_name();
            let name_str = name.to_string_lossy();

            // Remove .zst extension
            if let Some(commit_id) = name_str.strip_suffix(".zst")
                && commit_id.starts_with(prefix)
            {
                matches.push(commit_id.to_string());
            }
        }

        match matches.len() {
            0 => Ok(None),
            1 => Ok(Some(matches[0].clone())),
            _ => anyhow::bail!(
                "Ambiguous commit reference '{}' matches multiple commits: {}",
                prefix,
                matches.join(", ")
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{Commit, snapshots::Snapshot};
    use std::collections::HashMap;
    use std::fs;
    use std::path::Path;
    use tempfile::tempdir;

    fn setup_test_repo() -> Result<(tempfile::TempDir, RefResolver)> {
        let temp = tempdir()?;
        let repo_path = temp.path().join(".dotman");

        // Create repo structure
        fs::create_dir_all(&repo_path)?;
        fs::create_dir_all(repo_path.join("commits"))?;
        fs::create_dir_all(repo_path.join("refs/heads"))?;

        let resolver = RefResolver::new(repo_path.clone());

        // Initialize with main branch
        resolver.ref_manager.init()?;

        Ok((temp, resolver))
    }

    fn create_test_commit(repo_path: &Path, commit_id: &str, parent: Option<String>) -> Result<()> {
        let snapshot = Snapshot {
            commit: Commit {
                id: commit_id.to_string(),
                message: "Test commit".to_string(),
                timestamp: 1234567890,
                parent,
                author: "Test Author".to_string(),
                tree_hash: "test_tree".to_string(),
            },
            files: HashMap::new(),
        };

        // Save snapshot
        use crate::utils::compress::compress_bytes;
        use crate::utils::serialization::serialize;
        let serialized = serialize(&snapshot)?;
        let compressed = compress_bytes(&serialized, 3)?;
        let snapshot_path = repo_path.join("commits").join(format!("{}.zst", commit_id));
        fs::write(&snapshot_path, compressed)?;

        Ok(())
    }

    #[test]
    fn test_resolve_head() -> Result<()> {
        let (_temp, resolver) = setup_test_repo()?;

        // Create a commit and set HEAD
        let commit_id = "a".repeat(32);
        create_test_commit(&resolver.repo_path, &commit_id, None)?;
        resolver.ref_manager.update_branch("main", &commit_id)?;

        let resolved = resolver.resolve("HEAD")?;
        assert_eq!(resolved, commit_id);

        Ok(())
    }

    #[test]
    fn test_resolve_branch() -> Result<()> {
        let (_temp, resolver) = setup_test_repo()?;

        let commit_id = "b".repeat(32);
        create_test_commit(&resolver.repo_path, &commit_id, None)?;
        resolver
            .ref_manager
            .create_branch("feature", Some(&commit_id))?;

        let resolved = resolver.resolve("feature")?;
        assert_eq!(resolved, commit_id);

        Ok(())
    }

    #[test]
    fn test_resolve_full_commit() -> Result<()> {
        let (_temp, resolver) = setup_test_repo()?;

        let commit_id = "c".repeat(32);
        create_test_commit(&resolver.repo_path, &commit_id, None)?;

        let resolved = resolver.resolve(&commit_id)?;
        assert_eq!(resolved, commit_id);

        Ok(())
    }

    #[test]
    fn test_resolve_short_commit() -> Result<()> {
        let (_temp, resolver) = setup_test_repo()?;

        let commit_id = format!("d1234567{}", "0".repeat(24));
        create_test_commit(&resolver.repo_path, &commit_id, None)?;

        let resolved = resolver.resolve("d123")?;
        assert_eq!(resolved, commit_id);

        Ok(())
    }

    #[test]
    fn test_resolve_ambiguous_short_commit() -> Result<()> {
        let (_temp, resolver) = setup_test_repo()?;

        // Create two commits with same prefix
        let commit1 = format!("e1234567{}", "0".repeat(24));
        let commit2 = format!("e1234567{}", "1".repeat(24));
        create_test_commit(&resolver.repo_path, &commit1, None)?;
        create_test_commit(&resolver.repo_path, &commit2, None)?;

        let result = resolver.resolve("e123");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Ambiguous"));

        Ok(())
    }

    #[test]
    fn test_resolve_head_parent() -> Result<()> {
        let (_temp, resolver) = setup_test_repo()?;

        // Create a chain of commits
        let commit1 = format!("f1{}", "0".repeat(30));
        let commit2 = format!("f2{}", "0".repeat(30));
        let commit3 = format!("f3{}", "0".repeat(30));

        create_test_commit(&resolver.repo_path, &commit1, None)?;
        create_test_commit(&resolver.repo_path, &commit2, Some(commit1.clone()))?;
        create_test_commit(&resolver.repo_path, &commit3, Some(commit2.clone()))?;

        resolver.ref_manager.update_branch("main", &commit3)?;

        assert_eq!(resolver.resolve("HEAD")?, commit3);
        assert_eq!(resolver.resolve("HEAD~1")?, commit2);
        assert_eq!(resolver.resolve("HEAD~2")?, commit1);

        // Test going too far back
        let result = resolver.resolve("HEAD~3");
        assert!(result.is_err());

        Ok(())
    }

    #[test]
    fn test_resolve_ref_format() -> Result<()> {
        let (_temp, resolver) = setup_test_repo()?;

        let commit_id = "g".repeat(32);
        create_test_commit(&resolver.repo_path, &commit_id, None)?;
        resolver.ref_manager.update_branch("main", &commit_id)?;

        let resolved = resolver.resolve("ref: refs/heads/main")?;
        assert_eq!(resolved, commit_id);

        Ok(())
    }

    #[test]
    fn test_resolve_invalid_reference() -> Result<()> {
        let (_temp, resolver) = setup_test_repo()?;

        let result = resolver.resolve("invalid_ref");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Cannot resolve"));

        Ok(())
    }

    #[test]
    fn test_resolve_head_caret() -> Result<()> {
        let (_temp, resolver) = setup_test_repo()?;

        // Create a chain of commits
        let commit1 = format!("h1{}", "0".repeat(30));
        let commit2 = format!("h2{}", "0".repeat(30));
        let commit3 = format!("h3{}", "0".repeat(30));

        create_test_commit(&resolver.repo_path, &commit1, None)?;
        create_test_commit(&resolver.repo_path, &commit2, Some(commit1.clone()))?;
        create_test_commit(&resolver.repo_path, &commit3, Some(commit2.clone()))?;

        resolver.ref_manager.update_branch("main", &commit3)?;

        // Test HEAD^ (first parent)
        assert_eq!(resolver.resolve("HEAD^")?, commit2);
        
        // Test HEAD^^ (second ancestor)
        assert_eq!(resolver.resolve("HEAD^^")?, commit1);

        // Test HEAD^1 (equivalent to HEAD^)
        assert_eq!(resolver.resolve("HEAD^1")?, commit2);

        // Test HEAD^2 (equivalent to HEAD^^)
        assert_eq!(resolver.resolve("HEAD^2")?, commit1);

        Ok(())
    }

    #[test]
    fn test_resolve_head_caret_multiple() -> Result<()> {
        let (_temp, resolver) = setup_test_repo()?;

        // Create a longer chain of commits
        let commit1 = format!("i1{}", "0".repeat(30));
        let commit2 = format!("i2{}", "0".repeat(30));
        let commit3 = format!("i3{}", "0".repeat(30));
        let commit4 = format!("i4{}", "0".repeat(30));

        create_test_commit(&resolver.repo_path, &commit1, None)?;
        create_test_commit(&resolver.repo_path, &commit2, Some(commit1.clone()))?;
        create_test_commit(&resolver.repo_path, &commit3, Some(commit2.clone()))?;
        create_test_commit(&resolver.repo_path, &commit4, Some(commit3.clone()))?;

        resolver.ref_manager.update_branch("main", &commit4)?;

        // Test multiple carets
        assert_eq!(resolver.resolve("HEAD^^^")?, commit1);
        assert_eq!(resolver.resolve("HEAD^3")?, commit1);

        Ok(())
    }

    #[test]
    fn test_resolve_head_caret_equivalence() -> Result<()> {
        let (_temp, resolver) = setup_test_repo()?;

        // Create commits
        let commit1 = format!("j1{}", "0".repeat(30));
        let commit2 = format!("j2{}", "0".repeat(30));
        let commit3 = format!("j3{}", "0".repeat(30));

        create_test_commit(&resolver.repo_path, &commit1, None)?;
        create_test_commit(&resolver.repo_path, &commit2, Some(commit1.clone()))?;
        create_test_commit(&resolver.repo_path, &commit3, Some(commit2.clone()))?;

        resolver.ref_manager.update_branch("main", &commit3)?;

        // Verify equivalence between caret and tilde notation
        assert_eq!(resolver.resolve("HEAD^")?, resolver.resolve("HEAD~1")?);
        assert_eq!(resolver.resolve("HEAD^^")?, resolver.resolve("HEAD~2")?);
        assert_eq!(resolver.resolve("HEAD^1")?, resolver.resolve("HEAD~1")?);
        assert_eq!(resolver.resolve("HEAD^2")?, resolver.resolve("HEAD~2")?);

        Ok(())
    }

    #[test]
    fn test_resolve_head_caret_invalid() -> Result<()> {
        let (_temp, resolver) = setup_test_repo()?;

        // Test invalid caret notation
        let result = resolver.resolve("HEAD^xyz");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid parent specification"));

        // Test mixed caret and number
        let result = resolver.resolve("HEAD^2^");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid parent specification"));

        Ok(())
    }

    #[test]
    fn test_resolve_head_caret_beyond_history() -> Result<()> {
        let (_temp, resolver) = setup_test_repo()?;

        // Create only two commits
        let commit1 = format!("k1{}", "0".repeat(30));
        let commit2 = format!("k2{}", "0".repeat(30));

        create_test_commit(&resolver.repo_path, &commit1, None)?;
        create_test_commit(&resolver.repo_path, &commit2, Some(commit1.clone()))?;

        resolver.ref_manager.update_branch("main", &commit2)?;

        // Test going beyond available history
        let result = resolver.resolve("HEAD^^^");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Cannot go back"));

        let result = resolver.resolve("HEAD^3");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Cannot go back"));

        Ok(())
    }
}
