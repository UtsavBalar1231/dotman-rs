//! RAII-based transaction system for atomic repository operations.
//!
//! Provides automatic rollback on drop if not explicitly committed.
//! This ensures repository consistency even on panics or early returns.
//!
//! # Rollback Strategy
//!
//! The transaction system captures repository state at `begin()` and restores it
//! if the transaction is dropped without calling `commit()`. This is critical for
//! operations like `pull` that create multiple interdependent changes.
//!
//! ## What Gets Rolled Back (in order)
//!
//! 1. **Branch refs** - Restored to their values at transaction start. Direct file
//!    writes bypass validation since these commits existed at transaction start.
//!
//! 2. **Remote refs** - Updated refs restored to old values, newly created refs deleted.
//!    This prevents "phantom" remote tracking refs pointing to rolled-back commits.
//!
//! 3. **Orphaned commits** - Commit snapshots created during the transaction are deleted
//!    from `commits/`. These would otherwise be unreachable garbage.
//!
//! 4. **Orphaned mappings** - Git↔dotman commit mappings are removed to prevent
//!    the mapping file from referencing non-existent commits.
//!
//! 5. **Index** - Restored from backup file created at transaction start.
//!
//! ## Why This Order?
//!
//! Refs are restored first because they determine what's "reachable" - a commit
//! is only useful if a ref points to it. Commits and mappings are cleaned up after
//! refs so we don't delete something that's still referenced. Index is last because
//! it's least critical (staging area can be rebuilt).
//!
//! ## Error Handling Philosophy
//!
//! Rollback uses "continue on error" - we attempt all cleanup steps even if some
//! fail, collecting errors along the way. This maximizes recovery chances. Errors
//! are reported but don't stop subsequent cleanup steps.
//!
//! ## Partial Rollback Scenarios
//!
//! If rollback itself fails (e.g., permission error deleting a commit file):
//! - Repository may have orphaned commits (wasted space, harmless)
//! - Mapping file may reference non-existent commits (causes errors on lookup)
//! - User is warned and can run `dot fsck` to diagnose issues

use crate::DotmanContext;
use crate::INDEX_FILE;
use crate::mapping::MappingManager;
use crate::output;
use crate::refs::RefManager;
use anyhow::Result;
use std::collections::HashMap;
use std::path::PathBuf;

/// RAII Transaction Guard - auto-rollback on drop if not committed.
///
/// # Usage
///
/// ```ignore
/// let mut txn = Transaction::begin(ctx)?;
///
/// // Do risky operations...
/// txn.track_commit(&commit_id);
/// txn.track_mapping(remote, &commit_id, git_commit_id);
///
/// // If we get here, commit to prevent rollback
/// txn.commit()?;
/// ```
///
/// If the function returns early (error or panic) before `commit()`,
/// the `Drop` implementation will automatically rollback all tracked changes.
pub struct Transaction<'a> {
    /// Reference to the dotman context for repository operations.
    ctx: &'a DotmanContext,
    /// `None` after rollback completes (prevents double-rollback).
    checkpoint: Option<Checkpoint>,
    /// Set to `true` by `commit()` to prevent rollback on drop.
    committed: bool,
}

/// Repository state snapshot captured at transaction start.
struct Checkpoint {
    /// Restored on rollback to undo branch pointer changes.
    branch_refs: HashMap<String, String>,
    /// Restored on rollback; deleted on successful commit.
    index_backup: Option<PathBuf>,
    /// Deleted on rollback (orphaned snapshots from failed operation).
    created_commits: Vec<String>,
    /// Removed from mapping file on rollback (`remote`, `dotman_id`, `git_id`).
    created_mappings: Vec<(String, String, String)>,
    /// Restored on rollback ((`remote`, `branch`) -> `old_commit`).
    remote_refs: HashMap<(String, String), String>,
    /// Deleted on rollback (refs that didn't exist before transaction).
    created_remote_refs: Vec<(String, String)>,
}

impl<'a> Transaction<'a> {
    /// Begin a new transaction, capturing current repository state.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Failed to list current branches
    /// - Failed to backup the index file
    pub fn begin(ctx: &'a DotmanContext) -> Result<Self> {
        let ref_manager = RefManager::new(ctx.repo_path.clone());

        // Capture current branch refs
        let mut branch_refs = HashMap::new();
        for branch in ref_manager.list_branches()? {
            if let Ok(commit) = ref_manager.get_branch_commit(&branch) {
                branch_refs.insert(branch, commit);
            }
        }

        // Backup index if it exists
        let index_path = ctx.repo_path.join(INDEX_FILE);
        let backup_path = ctx.repo_path.join(".index.txn-backup");
        let index_backup = if index_path.exists() {
            std::fs::copy(&index_path, &backup_path)?;
            Some(backup_path)
        } else {
            None
        };

        Ok(Self {
            ctx,
            checkpoint: Some(Checkpoint {
                branch_refs,
                index_backup,
                created_commits: Vec::new(),
                created_mappings: Vec::new(),
                remote_refs: HashMap::new(),
                created_remote_refs: Vec::new(),
            }),
            committed: false,
        })
    }

    /// Track a commit created during this transaction.
    ///
    /// On rollback, this commit will be deleted from the commits directory.
    pub fn track_commit(&mut self, commit_id: &str) {
        if let Some(cp) = &mut self.checkpoint {
            cp.created_commits.push(commit_id.to_string());
        }
    }

    /// Track a mapping added during this transaction.
    ///
    /// On rollback, this mapping will be removed from the mapping file.
    pub fn track_mapping(&mut self, remote: &str, dotman_id: &str, git_id: &str) {
        if let Some(cp) = &mut self.checkpoint {
            cp.created_mappings.push((
                remote.to_string(),
                dotman_id.to_string(),
                git_id.to_string(),
            ));
        }
    }

    /// Track a remote ref update, storing the old value for rollback.
    ///
    /// Call this BEFORE updating the ref. Pass `None` if the ref didn't exist
    /// (newly created refs will be deleted on rollback).
    pub fn track_remote_ref(&mut self, remote: &str, branch: &str, old_commit: Option<&str>) {
        if let Some(cp) = &mut self.checkpoint {
            if let Some(old) = old_commit {
                // Existing ref - restore to old value on rollback
                cp.remote_refs
                    .insert((remote.to_string(), branch.to_string()), old.to_string());
            } else {
                // New ref - delete on rollback
                cp.created_remote_refs
                    .push((remote.to_string(), branch.to_string()));
            }
        }
    }

    /// Commit the transaction, preventing automatic rollback.
    ///
    /// Call this when all operations have succeeded.
    /// After this call, the Transaction will not rollback on drop.
    ///
    /// # Errors
    ///
    /// This function is infallible but returns `Result` for API consistency.
    pub fn commit(mut self) -> Result<()> {
        self.committed = true;
        self.cleanup_backup();
        Ok(())
    }

    /// Clean up backup files (called on successful commit).
    fn cleanup_backup(&self) {
        if let Some(cp) = &self.checkpoint
            && let Some(backup) = &cp.index_backup
        {
            let _ = std::fs::remove_file(backup);
        }
    }

    /// Perform rollback to restore repository to checkpoint state.
    ///
    /// Implements a multi-phase recovery process:
    /// 1. Restore branch refs to pre-transaction values
    /// 2. Restore remote refs to original values
    /// 3. Delete newly created remote refs
    /// 4. Delete orphaned commit snapshots
    /// 5. Remove orphaned git↔dotman mappings
    /// 6. Restore index from backup
    ///
    /// Uses "continue on error" strategy - attempts all phases even if some fail,
    /// collecting errors along the way to maximize recovery.
    ///
    /// # Errors
    ///
    /// Returns an error if any rollback step fails. The error message lists all
    /// failures, but subsequent steps still execute regardless of earlier failures.
    fn rollback(&mut self) -> Result<()> {
        let Some(checkpoint) = self.checkpoint.take() else {
            return Ok(());
        };

        let ref_manager = RefManager::new(self.ctx.repo_path.clone());
        let mut errors = Vec::new();

        // 1. Restore branch refs to original values (direct write, no validation)
        // We skip validation because this is disaster recovery - the commit existed
        // at transaction start, so we restore unconditionally.
        for (branch, commit) in &checkpoint.branch_refs {
            let branch_path = self.ctx.repo_path.join(format!("refs/heads/{branch}"));
            if let Err(e) = std::fs::write(&branch_path, commit) {
                errors.push(format!("branch '{branch}': {e}"));
            }
        }

        // 2. Restore remote refs to original values (direct write, no validation)
        for ((remote, branch), commit) in &checkpoint.remote_refs {
            let ref_path = self
                .ctx
                .repo_path
                .join(format!("refs/remotes/{remote}/{branch}"));
            if let Some(parent) = ref_path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if let Err(e) = std::fs::write(&ref_path, commit) {
                errors.push(format!("remote '{remote}/{branch}': {e}"));
            }
        }

        // 2b. Delete newly created remote refs
        for (remote, branch) in &checkpoint.created_remote_refs {
            if let Err(e) = ref_manager.delete_remote_ref(remote, branch) {
                errors.push(format!("delete remote '{remote}/{branch}': {e}"));
            }
        }

        // 3. Delete orphaned commits
        for commit_id in &checkpoint.created_commits {
            let path = self
                .ctx
                .repo_path
                .join("commits")
                .join(format!("{commit_id}.zst"));
            let short_id = &commit_id[..8.min(commit_id.len())];
            if path.exists()
                && let Err(e) = std::fs::remove_file(&path)
            {
                errors.push(format!("commit {short_id}: {e}"));
            }
        }

        // 4. Remove orphaned mappings
        if !checkpoint.created_mappings.is_empty()
            && let Ok(mut mm) = MappingManager::new(&self.ctx.repo_path)
        {
            for (remote, dotman_id, git_id) in &checkpoint.created_mappings {
                let _ = mm.remove_and_save(remote, dotman_id, git_id);
            }
        }

        // 5. Restore index from backup
        if let Some(backup) = &checkpoint.index_backup
            && backup.exists()
        {
            let index_path = self.ctx.repo_path.join(INDEX_FILE);
            if let Err(e) = std::fs::copy(backup, &index_path) {
                errors.push(format!("index: {e}"));
            }
            let _ = std::fs::remove_file(backup);
        }

        if errors.is_empty() {
            output::info("Transaction rolled back successfully");
            Ok(())
        } else {
            Err(anyhow::anyhow!("Rollback errors: {}", errors.join("; ")))
        }
    }
}

impl Drop for Transaction<'_> {
    fn drop(&mut self) {
        if !self.committed && self.checkpoint.is_some() {
            output::warning("Transaction not committed, rolling back...");
            if let Err(e) = self.rollback() {
                output::error(&format!("Rollback failed: {e}"));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup_test_context() -> (TempDir, DotmanContext) {
        let temp = TempDir::new().unwrap();
        let repo_path = temp.path().join(".dotman");
        let config_path = temp.path().join("config.toml");
        std::fs::create_dir_all(&repo_path).unwrap();
        std::fs::create_dir_all(repo_path.join("refs/heads")).unwrap();
        std::fs::create_dir_all(repo_path.join("refs/remotes/origin")).unwrap();
        std::fs::create_dir_all(repo_path.join("commits")).unwrap();

        let ctx = DotmanContext::new_explicit(repo_path, config_path).unwrap();
        (temp, ctx)
    }

    #[test]
    fn test_transaction_commit_prevents_rollback() {
        let (_temp, ctx) = setup_test_context();

        // Create a transaction and commit it
        let txn = Transaction::begin(&ctx).unwrap();
        txn.commit().unwrap();
        // No rollback should happen
    }

    #[test]
    fn test_transaction_drop_triggers_rollback() {
        let (_temp, ctx) = setup_test_context();

        // Create index file
        let index_path = ctx.repo_path.join(INDEX_FILE);
        std::fs::write(&index_path, b"original").unwrap();

        {
            let mut txn = Transaction::begin(&ctx).unwrap();

            // Modify index
            std::fs::write(&index_path, b"modified").unwrap();

            // Track a fake commit
            txn.track_commit("abc123");

            // Drop without commit - should trigger rollback
        }

        // Index should be restored
        let content = std::fs::read_to_string(&index_path).unwrap();
        assert_eq!(content, "original");
    }

    #[test]
    fn test_branch_ref_restoration_on_rollback() {
        let (_temp, ctx) = setup_test_context();

        // Create a branch ref file directly (bypass validation for testing)
        let ref_manager = RefManager::new(ctx.repo_path.clone());
        let original_commit = "abc123def456abc123def456abc123def456abc123";
        let branch_path = ctx.repo_path.join("refs/heads/main");
        std::fs::write(&branch_path, original_commit).unwrap();

        {
            let _txn = Transaction::begin(&ctx).unwrap();

            // Modify branch ref directly
            let new_commit = "999888777666999888777666999888777666999888";
            std::fs::write(&branch_path, new_commit).unwrap();

            // Verify it changed
            assert_eq!(ref_manager.get_branch_commit("main").unwrap(), new_commit);

            // Drop without commit - should trigger rollback
        }

        // Branch ref should be restored
        assert_eq!(
            ref_manager.get_branch_commit("main").unwrap(),
            original_commit
        );
    }

    #[test]
    fn test_remote_ref_restoration_on_rollback() {
        let (_temp, ctx) = setup_test_context();

        // Create a remote ref
        let ref_manager = RefManager::new(ctx.repo_path.clone());
        let original_commit = "remote123456";
        ref_manager
            .update_remote_ref("origin", "main", original_commit)
            .unwrap();

        {
            let mut txn = Transaction::begin(&ctx).unwrap();

            // Track the remote ref update BEFORE modifying
            txn.track_remote_ref("origin", "main", Some(original_commit));

            // Modify remote ref
            let new_commit = "newremote789";
            ref_manager
                .update_remote_ref("origin", "main", new_commit)
                .unwrap();

            // Verify it changed
            assert_eq!(
                ref_manager.get_remote_ref("origin", "main").unwrap(),
                new_commit
            );

            // Drop without commit - should trigger rollback
        }

        // Remote ref should be restored
        assert_eq!(
            ref_manager.get_remote_ref("origin", "main").unwrap(),
            original_commit
        );
    }

    #[test]
    fn test_newly_created_remote_ref_deleted_on_rollback() {
        let (_temp, ctx) = setup_test_context();

        let ref_manager = RefManager::new(ctx.repo_path.clone());

        // Verify remote ref doesn't exist
        assert!(!ref_manager.remote_ref_exists("origin", "feature"));

        {
            let mut txn = Transaction::begin(&ctx).unwrap();

            // Track as newly created (None = no previous value)
            txn.track_remote_ref("origin", "feature", None);

            // Create the remote ref
            ref_manager
                .update_remote_ref("origin", "feature", "newcommit123")
                .unwrap();

            // Verify it was created
            assert!(ref_manager.remote_ref_exists("origin", "feature"));

            // Drop without commit - should trigger rollback
        }

        // Newly created remote ref should be deleted
        assert!(!ref_manager.remote_ref_exists("origin", "feature"));
    }

    #[test]
    fn test_commit_deletion_on_rollback() {
        let (_temp, ctx) = setup_test_context();

        let commit_id = "deadbeef12345678";
        let commit_path = ctx
            .repo_path
            .join("commits")
            .join(format!("{commit_id}.zst"));

        {
            let mut txn = Transaction::begin(&ctx).unwrap();

            // Create a fake commit file
            std::fs::write(&commit_path, b"fake commit data").unwrap();
            assert!(commit_path.exists());

            // Track the commit
            txn.track_commit(commit_id);

            // Drop without commit - should trigger rollback
        }

        // Commit file should be deleted
        assert!(!commit_path.exists());
    }

    #[test]
    fn test_committed_transaction_preserves_changes() {
        let (_temp, ctx) = setup_test_context();

        let ref_manager = RefManager::new(ctx.repo_path.clone());
        let commit_id = "preserved123456preserved123456preserved1234";
        let commit_path = ctx
            .repo_path
            .join("commits")
            .join(format!("{commit_id}.zst"));

        {
            let mut txn = Transaction::begin(&ctx).unwrap();

            // Create branch ref directly (bypass validation for testing)
            let branch_path = ctx.repo_path.join("refs/heads/feature");
            std::fs::write(&branch_path, commit_id).unwrap();

            // Create commit file
            std::fs::write(&commit_path, b"preserved commit").unwrap();
            txn.track_commit(commit_id);

            // Create remote ref
            txn.track_remote_ref("origin", "feature", None);
            ref_manager
                .update_remote_ref("origin", "feature", commit_id)
                .unwrap();

            // Commit the transaction
            txn.commit().unwrap();
        }

        // All changes should be preserved
        assert_eq!(ref_manager.get_branch_commit("feature").unwrap(), commit_id);
        assert!(commit_path.exists());
        assert!(ref_manager.remote_ref_exists("origin", "feature"));
    }

    #[test]
    fn test_multiple_remote_refs_rollback() {
        let (_temp, ctx) = setup_test_context();

        let ref_manager = RefManager::new(ctx.repo_path.clone());

        // Create one existing ref
        ref_manager
            .update_remote_ref("origin", "main", "existing123")
            .unwrap();

        {
            let mut txn = Transaction::begin(&ctx).unwrap();

            // Track existing ref update
            txn.track_remote_ref("origin", "main", Some("existing123"));
            ref_manager
                .update_remote_ref("origin", "main", "updated456")
                .unwrap();

            // Track new ref creation
            txn.track_remote_ref("origin", "develop", None);
            ref_manager
                .update_remote_ref("origin", "develop", "newdev789")
                .unwrap();

            // Track another new ref
            txn.track_remote_ref("origin", "feature", None);
            ref_manager
                .update_remote_ref("origin", "feature", "newfeat000")
                .unwrap();

            // Drop without commit
        }

        // Existing ref should be restored to original value
        assert_eq!(
            ref_manager.get_remote_ref("origin", "main").unwrap(),
            "existing123"
        );

        // New refs should be deleted
        assert!(!ref_manager.remote_ref_exists("origin", "develop"));
        assert!(!ref_manager.remote_ref_exists("origin", "feature"));
    }

    #[test]
    fn test_index_backup_cleanup_on_commit() {
        let (_temp, ctx) = setup_test_context();

        let index_path = ctx.repo_path.join(INDEX_FILE);
        let backup_path = ctx.repo_path.join(".index.txn-backup");

        // Create index file
        std::fs::write(&index_path, b"index data").unwrap();

        {
            let txn = Transaction::begin(&ctx).unwrap();

            // Backup should exist
            assert!(backup_path.exists());

            // Commit
            txn.commit().unwrap();
        }

        // Backup should be cleaned up after commit
        assert!(!backup_path.exists());
    }
}
