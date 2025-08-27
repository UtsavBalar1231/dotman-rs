use crate::refs::resolver::RefResolver;
use crate::storage::FileStatus;
use crate::storage::index::{Index, IndexDiffer};
use crate::storage::snapshots::SnapshotManager;
use crate::utils::pager::PagerOutput;
use crate::{DotmanContext, INDEX_FILE};
use anyhow::{Context, Result};
use colored::Colorize;

pub fn execute(ctx: &DotmanContext, from: Option<&str>, to: Option<&str>) -> Result<()> {
    ctx.check_repo_initialized()?;

    match (from, to) {
        (None, None) => {
            // Diff working directory against index
            diff_working_vs_index(ctx)
        }
        (Some(commit), None) => {
            // Diff commit against working directory
            diff_commit_vs_working(ctx, commit)
        }
        (Some(from_commit), Some(to_commit)) => {
            // Diff between two commits
            diff_commits(ctx, from_commit, to_commit)
        }
        _ => anyhow::bail!("Invalid diff arguments"),
    }
}

fn diff_working_vs_index(ctx: &DotmanContext) -> Result<()> {
    use crate::commands::status::get_current_files;
    use crate::storage::index::ConcurrentIndex;

    let mut output = PagerOutput::default();
    output.appendln(&format!(
        "{}",
        "Comparing working directory with index...".blue()
    ));

    let index_path = ctx.repo_path.join(INDEX_FILE);
    let index = Index::load(&index_path)?;
    let concurrent_index = ConcurrentIndex::from_index(index);

    let current_files = get_current_files(ctx)?;
    let statuses = concurrent_index.get_status_parallel(&current_files);

    if statuses.is_empty() {
        output.appendln("No differences found");
        // output.show()?;
        return Ok(());
    }

    format_file_statuses(&mut output, &statuses);
    // output.show()?;

    Ok(())
}

fn diff_commit_vs_working(ctx: &DotmanContext, commit: &str) -> Result<()> {
    // Resolve the commit reference
    let resolver = RefResolver::new(ctx.repo_path.clone());
    let commit_id = resolver
        .resolve(commit)
        .with_context(|| format!("Failed to resolve reference: {}", commit))?;

    let mut output = PagerOutput::default();
    output.appendln(&format!(
        "{}",
        format!(
            "Comparing commit {} with working directory...",
            commit_id[..8.min(commit_id.len())].yellow()
        )
        .blue()
    ));

    let snapshot_manager =
        SnapshotManager::new(ctx.repo_path.clone(), ctx.config.core.compression_level);

    let snapshot = snapshot_manager
        .load_snapshot(&commit_id)
        .with_context(|| format!("Failed to load commit: {}", commit_id))?;

    // Convert snapshot to index format for comparison
    let mut commit_index = Index::new();
    for (path, file) in &snapshot.files {
        commit_index.add_entry(crate::storage::FileEntry {
            path: path.clone(),
            hash: file.hash.clone(),
            size: 0,
            modified: snapshot.commit.timestamp,
            mode: file.mode,
        });
    }

    // Get current working directory state
    let index_path = ctx.repo_path.join(INDEX_FILE);
    let working_index = Index::load(&index_path)?;

    let statuses = IndexDiffer::diff(&commit_index, &working_index);

    if statuses.is_empty() {
        output.appendln("No differences found");
        // output.show()?;
        return Ok(());
    }

    format_file_statuses(&mut output, &statuses);
    // output.show()?;

    Ok(())
}

fn diff_commits(ctx: &DotmanContext, from: &str, to: &str) -> Result<()> {
    // Resolve the commit references
    let resolver = RefResolver::new(ctx.repo_path.clone());
    let from_id = resolver
        .resolve(from)
        .with_context(|| format!("Failed to resolve reference: {}", from))?;
    let to_id = resolver
        .resolve(to)
        .with_context(|| format!("Failed to resolve reference: {}", to))?;

    let mut output = PagerOutput::default();
    output.appendln(&format!(
        "{}",
        format!(
            "Comparing commit {} with commit {}...",
            from_id[..8.min(from_id.len())].yellow(),
            to_id[..8.min(to_id.len())].yellow()
        )
        .blue()
    ));

    let snapshot_manager =
        SnapshotManager::new(ctx.repo_path.clone(), ctx.config.core.compression_level);

    let from_snapshot = snapshot_manager
        .load_snapshot(&from_id)
        .with_context(|| format!("Failed to load commit: {}", from_id))?;
    let to_snapshot = snapshot_manager
        .load_snapshot(&to_id)
        .with_context(|| format!("Failed to load commit: {}", to_id))?;

    // Convert snapshots to index format
    let mut from_index = Index::new();
    for (path, file) in &from_snapshot.files {
        from_index.add_entry(crate::storage::FileEntry {
            path: path.clone(),
            hash: file.hash.clone(),
            size: 0,
            modified: from_snapshot.commit.timestamp,
            mode: file.mode,
        });
    }

    let mut to_index = Index::new();
    for (path, file) in &to_snapshot.files {
        to_index.add_entry(crate::storage::FileEntry {
            path: path.clone(),
            hash: file.hash.clone(),
            size: 0,
            modified: to_snapshot.commit.timestamp,
            mode: file.mode,
        });
    }

    let statuses = IndexDiffer::diff(&from_index, &to_index);

    if statuses.is_empty() {
        output.appendln("No differences found");
        output.show()?;
        return Ok(());
    }

    format_file_statuses(&mut output, &statuses);
    // output.show()?;

    Ok(())
}

fn format_file_statuses(output: &mut PagerOutput, statuses: &[FileStatus]) {
    let mut added = Vec::new();
    let mut modified = Vec::new();
    let mut deleted = Vec::new();

    for status in statuses {
        match status {
            FileStatus::Added(p) => added.push(p),
            FileStatus::Modified(p) => modified.push(p),
            FileStatus::Deleted(p) => deleted.push(p),
            FileStatus::Untracked(p) => added.push(p), // Treat untracked as added in diff
        }
    }

    if !added.is_empty() {
        output.appendln("");
        output.appendln(&format!("{}", "Added files:".green().bold()));
        for path in &added {
            output.appendln(&format!("  + {}", path.display()));
        }
    }

    if !modified.is_empty() {
        output.appendln("");
        output.appendln(&format!("{}", "Modified files:".yellow().bold()));
        for path in &modified {
            output.appendln(&format!("  ~ {}", path.display()));
        }
    }

    if !deleted.is_empty() {
        output.appendln("");
        output.appendln(&format!("{}", "Deleted files:".red().bold()));
        for path in &deleted {
            output.appendln(&format!("  - {}", path.display()));
        }
    }

    output.appendln("");
    output.appendln(&format!(
        "{}: {} added, {} modified, {} deleted",
        "Summary".bold(),
        added.len(),
        modified.len(),
        deleted.len()
    ));
}

// Keep the old function for tests
#[allow(dead_code)]
fn display_file_statuses(statuses: &[FileStatus]) {
    let mut output = PagerOutput::default();
    format_file_statuses(&mut output, statuses);
    let _ = output.disable_pager().show();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use std::fs;
    use tempfile::tempdir;

    fn setup_test_context() -> Result<(tempfile::TempDir, DotmanContext)> {
        let temp = tempdir()?;
        let repo_path = temp.path().join(".dotman");
        let config_path = temp.path().join("config.toml");

        // Create repo structure
        fs::create_dir_all(&repo_path)?;
        fs::create_dir_all(repo_path.join("commits"))?;
        fs::create_dir_all(repo_path.join("objects"))?;

        // Create empty index
        let index = Index::new();
        let index_path = repo_path.join("index.bin");
        index.save(&index_path)?;

        // Create HEAD file (required for repo initialization check)
        fs::write(repo_path.join("HEAD"), "")?;

        let mut config = Config::default();
        config.core.repo_path = repo_path.clone();
        config.save(&config_path)?;

        let ctx = DotmanContext {
            repo_path,
            config_path,
            config,
        };

        Ok((temp, ctx))
    }

    #[test]
    #[serial_test::serial]
    fn test_execute_no_args() -> Result<()> {
        let (_temp, ctx) = setup_test_context()?;

        // Set HOME for tests
        unsafe {
            std::env::set_var("HOME", _temp.path());
        }

        let result = execute(&ctx, None, None);
        assert!(result.is_ok());

        Ok(())
    }

    #[test]
    #[serial_test::serial]
    fn test_execute_one_commit() -> Result<()> {
        let (_temp, ctx) = setup_test_context()?;

        unsafe {
            std::env::set_var("HOME", _temp.path());
        }

        // Test diff with one commit (commit vs working)
        let result = execute(&ctx, Some("HEAD"), None);
        // Will fail if HEAD doesn't exist, which is expected
        assert!(result.is_err());

        Ok(())
    }

    #[test]
    fn test_execute_two_commits() -> Result<()> {
        let (_temp, ctx) = setup_test_context()?;

        // Test diff between two commits
        let result = execute(&ctx, Some("commit1"), Some("commit2"));
        // Will fail if commits don't exist, which is expected
        assert!(result.is_err());

        Ok(())
    }

    #[test]
    #[serial_test::serial]
    fn test_diff_working_vs_index_no_changes() -> Result<()> {
        let (_temp, ctx) = setup_test_context()?;

        unsafe {
            std::env::set_var("HOME", _temp.path());
        }

        let result = diff_working_vs_index(&ctx);
        assert!(result.is_ok());

        Ok(())
    }

    #[test]
    fn test_display_file_statuses() {
        use std::path::PathBuf;

        let statuses = vec![
            FileStatus::Added(PathBuf::from("new.txt")),
            FileStatus::Modified(PathBuf::from("changed.txt")),
            FileStatus::Deleted(PathBuf::from("removed.txt")),
            FileStatus::Untracked(PathBuf::from("unknown.txt")),
        ];

        // This function just prints, so we're testing it doesn't panic
        display_file_statuses(&statuses);
    }

    #[test]
    fn test_display_empty_file_statuses() {
        let statuses = vec![];
        display_file_statuses(&statuses);
    }

    #[test]
    fn test_check_repo_initialized() -> Result<()> {
        let temp = tempdir()?;
        let repo_path = temp.path().join(".dotman");

        let ctx = DotmanContext {
            repo_path: repo_path.clone(),
            config_path: temp.path().join("config"),
            config: Config::default(),
        };

        let result = ctx.check_repo_initialized();
        assert!(result.is_err());

        Ok(())
    }

    #[test]
    fn test_diff_with_special_characters() {
        use std::path::PathBuf;

        let statuses = vec![
            FileStatus::Added(PathBuf::from("文件.txt")),
            FileStatus::Modified(PathBuf::from("ñoño.conf")),
            FileStatus::Deleted(PathBuf::from("🎉emoji.rs")),
        ];

        display_file_statuses(&statuses);
    }

    #[test]
    fn test_diff_with_long_paths() {
        use std::path::PathBuf;

        let long_path = "a/".repeat(50) + "file.txt";
        let statuses = vec![FileStatus::Modified(PathBuf::from(long_path))];

        display_file_statuses(&statuses);
    }
}
