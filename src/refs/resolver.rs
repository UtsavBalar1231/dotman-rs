use crate::NULL_COMMIT_ID;
use crate::refs::RefManager;
use crate::storage::snapshots::SnapshotManager;
use anyhow::{Context, Result};
use std::path::PathBuf;

/// Resolves various reference formats to commit IDs
pub struct RefResolver {
    /// Path to the dotman repository
    repo_path: PathBuf,
    /// Reference manager for accessing branch and tag information
    ref_manager: RefManager,
}

impl RefResolver {
    /// Creates a new reference resolver for a repository
    ///
    /// # Arguments
    ///
    /// * `repo_path` - Path to the dotman repository directory
    ///
    /// # Returns
    ///
    /// Returns a new `RefResolver` instance configured for the specified repository
    #[must_use]
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
    /// - Tag names
    /// - Full commit IDs
    /// - Short commit IDs (prefix matching)
    /// - ref: refs/heads/branch format
    /// - ref: refs/tags/tag format
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The reference cannot be resolved
    /// - The referenced commit does not exist
    /// - The reference format is invalid
    pub fn resolve(&self, reference: &str) -> Result<String> {
        if let Some(branch) = reference.strip_prefix("ref: refs/heads/") {
            return self.resolve_branch(branch);
        }

        if let Some(tag) = reference.strip_prefix("ref: refs/tags/") {
            return self.resolve_tag(tag);
        }

        if reference == "HEAD" {
            return self.resolve_head();
        }

        if let Some(parent_spec) = reference.strip_prefix("HEAD~") {
            let parent_count = parent_spec
                .parse::<usize>()
                .with_context(|| format!("Invalid parent specification: {reference}"))?;
            return self.resolve_head_parent(parent_count);
        }

        if let Some(caret_spec) = reference.strip_prefix("HEAD^") {
            // Parse caret notation - distinguishes between HEAD^n (nth parent) and HEAD^^ (ancestors)
            return self.resolve_caret_notation(caret_spec, reference);
        }

        // Try as remote ref (e.g., "origin/main")
        if let Some((remote, branch)) = reference.split_once('/')
            && self.ref_manager.remote_ref_exists(remote, branch)
        {
            return self.resolve_remote_ref(remote, branch);
        }

        // Try as branch name
        if self.ref_manager.branch_exists(reference) {
            return self.resolve_branch(reference);
        }

        // Try as tag name
        if self.ref_manager.tag_exists(reference) {
            return self.resolve_tag(reference);
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

        Err(anyhow::anyhow!("Cannot resolve reference: {reference}"))
    }

    /// Resolve HEAD to current commit
    fn resolve_head(&self) -> Result<String> {
        self.ref_manager
            .get_head_commit()?
            .context("No commits yet")
    }

    /// Resolve HEAD~n to nth ancestor following first parent chain
    fn resolve_head_parent(&self, ancestor_count: usize) -> Result<String> {
        if ancestor_count == 0 {
            return self.resolve_head();
        }

        let mut current = self.resolve_head()?;
        let snapshot_manager = SnapshotManager::new(self.repo_path.clone(), 3);

        for i in 0..ancestor_count {
            let Ok(snapshot) = snapshot_manager.load_snapshot(&current) else {
                return Err(anyhow::anyhow!(
                    "Cannot go back {ancestor_count} commits from HEAD (only {i} commits in history)"
                ));
            };

            if let Some(parent) = snapshot.commit.parents.first() {
                if parent == NULL_COMMIT_ID {
                    return Err(Self::ancestor_error(ancestor_count, i));
                }
                current.clone_from(parent);
            } else {
                // No parents means root commit
                return Err(Self::ancestor_error(ancestor_count, i));
            }
        }

        Ok(current)
    }

    /// Generate appropriate error for ancestor traversal failure
    fn ancestor_error(requested: usize, reached: usize) -> anyhow::Error {
        if reached == 0 {
            anyhow::anyhow!(
                "Cannot go back {} commit{} from HEAD: current commit is the initial commit",
                requested,
                if requested == 1 { "" } else { "s" }
            )
        } else {
            anyhow::anyhow!(
                "Cannot go back {} commits from HEAD: only {} commit{} in history before HEAD",
                requested,
                reached,
                if reached == 1 { "" } else { "s" }
            )
        }
    }

    /// Resolve caret notation (HEAD^, HEAD^2, HEAD^^)
    ///
    /// Git semantics:
    /// - HEAD^ or HEAD^1 = first parent
    /// - HEAD^2 = second parent (for merge commits)
    /// - HEAD^^ = HEAD^1^1 = first parent of first parent
    fn resolve_caret_notation(&self, caret_spec: &str, full_reference: &str) -> Result<String> {
        let snapshot_manager = SnapshotManager::new(self.repo_path.clone(), 3);

        if caret_spec.is_empty() {
            // HEAD^ means first parent
            return Self::resolve_nth_parent(&snapshot_manager, &self.resolve_head()?, 1);
        }

        if caret_spec.chars().all(|c| c == '^') {
            // HEAD^^ = first parent of first parent = HEAD~2
            // HEAD^^^ = HEAD~3
            return self.resolve_head_parent(caret_spec.len() + 1);
        }

        if let Ok(parent_index) = caret_spec.parse::<usize>() {
            // HEAD^2 = second parent of HEAD
            return Self::resolve_nth_parent(
                &snapshot_manager,
                &self.resolve_head()?,
                parent_index,
            );
        }

        Err(anyhow::anyhow!(
            "Invalid parent specification: {full_reference}"
        ))
    }

    /// Resolve the nth parent of a commit (1-indexed, like git)
    fn resolve_nth_parent(
        snapshot_manager: &SnapshotManager,
        commit_id: &str,
        parent_index: usize,
    ) -> Result<String> {
        let snapshot = snapshot_manager
            .load_snapshot(commit_id)
            .with_context(|| format!("Failed to load commit: {commit_id}"))?;

        if parent_index == 0 {
            return Err(anyhow::anyhow!("Parent index must be >= 1"));
        }

        // Convert 1-indexed to 0-indexed
        let idx = parent_index - 1;

        snapshot
            .commit
            .parents
            .get(idx)
            .filter(|p| *p != NULL_COMMIT_ID)
            .cloned()
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Commit {} has no parent #{} (has {} parent{})",
                    &commit_id[..8.min(commit_id.len())],
                    parent_index,
                    snapshot.commit.parents.len(),
                    if snapshot.commit.parents.len() == 1 {
                        ""
                    } else {
                        "s"
                    }
                )
            })
    }

    /// Resolve a branch name to commit ID
    fn resolve_branch(&self, branch: &str) -> Result<String> {
        self.ref_manager.get_branch_commit(branch)
    }

    /// Resolve a tag name to commit ID
    fn resolve_tag(&self, tag: &str) -> Result<String> {
        self.ref_manager.get_tag_commit(tag)
    }

    /// Resolve a remote ref to commit ID
    fn resolve_remote_ref(&self, remote: &str, branch: &str) -> Result<String> {
        self.ref_manager.get_remote_ref(remote, branch)
    }

    /// Find a commit by prefix (short hash) with validation
    ///
    /// This function implements prefix matching for short commit IDs, similar to git.
    /// It validates that the prefix:
    /// 1. Is unambiguous (matches exactly one commit)
    /// 2. Corresponds to an actual commit file on disk
    ///
    /// ## Prefix Matching Strategy
    ///
    /// Dotman commit IDs are 32-character hex strings (MD5-sized), so we support
    /// short references like `abc1234` instead of requiring the full
    /// `abc1234567890abcdef1234567890ab`.
    ///
    /// The matching process:
    /// 1. Scan all commit files in `commits/` directory
    /// 2. Check each filename (minus `.zst` extension) for prefix match
    /// 3. Collect all matches
    /// 4. Validate uniqueness
    ///
    /// ## Ambiguity Detection
    ///
    /// If multiple commits start with the same prefix, we cannot determine which
    /// the user meant. This is reported as an error with the full list of matches,
    /// allowing the user to provide a longer prefix.
    ///
    /// Example:
    /// ```text
    /// abc1234567890... (commit 1)
    /// abc1234ABCDEF... (commit 2)
    /// ```
    ///
    /// Prefix `abc1` is ambiguous and will fail. User must use at least `abc12345`
    /// or `abc1234A` to disambiguate.
    ///
    /// ## Minimum Prefix Length
    ///
    /// The caller enforces a minimum prefix length (typically 4 chars) before calling
    /// this function. This prevents expensive directory scans for very short prefixes
    /// that are likely to be ambiguous.
    ///
    /// ## Performance
    ///
    /// This function does a linear scan of the commits directory, which is acceptable
    /// for typical repository sizes (hundreds to thousands of commits). For very large
    /// repositories (10,000+ commits), an in-memory index optimization is available
    /// (see issue #4). Current implementation prioritizes simplicity.
    fn find_commit_by_prefix(&self, prefix: &str) -> Result<Option<String>> {
        let commits_dir = self.repo_path.join("commits");
        if !commits_dir.exists() {
            return Ok(None);
        }

        let mut matches = Vec::new();

        // Scan commits directory for files matching the prefix
        for entry in std::fs::read_dir(&commits_dir)? {
            let entry = entry?;
            let name = entry.file_name();
            let name_str = name.to_string_lossy();

            // Extract commit ID from filename (remove .zst extension)
            if let Some(commit_id) = name_str.strip_suffix(".zst")
                && commit_id.starts_with(prefix)
            {
                matches.push(commit_id.to_string());
            }
        }

        // Validate match count and return result
        match matches.len() {
            0 => Ok(None),
            1 => Ok(Some(matches[0].clone())),
            _ => {
                // Multiple matches - ambiguous reference
                // Show user all matches so they can choose a longer prefix
                Err(anyhow::anyhow!(
                    "Ambiguous commit reference '{}' matches multiple commits: {}",
                    prefix,
                    matches.join(", ")
                ))
            }
        }
    }
}
