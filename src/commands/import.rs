use crate::DotmanContext;
use anyhow::{Context, Result};
use colored::Colorize;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;
use walkdir::WalkDir;

pub fn execute(
    ctx: &DotmanContext,
    source: &str,
    track: bool,
    force: bool,
    dry_run: bool,
    yes: bool,
) -> Result<()> {
    ctx.check_repo_initialized()?;

    super::print_info(&format!("Importing dotfiles from: {}", source));

    // Step 1: Determine source type and prepare repository
    let (repo_path, _temp_dir) = if source.starts_with("http") || source.starts_with("git@") {
        // Clone to temporary directory
        super::print_info("Cloning remote repository...");
        let temp_dir = TempDir::new()?;
        let repo_path = temp_dir.path().to_path_buf();

        clone_repository(source, &repo_path)?;

        // Return both path and TempDir to keep it alive
        (repo_path, Some(temp_dir))
    } else {
        // Use local path
        let path = PathBuf::from(source);
        if !path.exists() {
            anyhow::bail!("Source path does not exist: {}", source);
        }
        if !path.is_dir() {
            anyhow::bail!("Source path is not a directory: {}", source);
        }
        (path, None)
    };

    // Step 2: Scan all files in repository
    let files_to_import = scan_repository(&repo_path)?;

    if files_to_import.is_empty() {
        super::print_warning("No files found to import");
        return Ok(());
    }

    super::print_info(&format!(
        "Found {} file{} to import",
        files_to_import.len(),
        if files_to_import.len() == 1 { "" } else { "s" }
    ));

    // Step 3: Check for conflicts if not forcing
    let conflicts = if !force {
        check_existing_files(&files_to_import)?
    } else {
        vec![]
    };

    if !conflicts.is_empty() && !force {
        super::print_warning(&format!(
            "Found {} existing file{} that would be overwritten:",
            conflicts.len(),
            if conflicts.len() == 1 { "" } else { "s" }
        ));

        for (_, target) in &conflicts {
            println!("  {}", target.display().to_string().yellow());
        }

        if !yes {
            // Ask for confirmation
            println!();
            print!("Do you want to overwrite these files? [y/N]: ");
            use std::io::{self, Write};
            io::stdout().flush()?;

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;

            if !input.trim().eq_ignore_ascii_case("y") {
                super::print_info("Import cancelled");
                return Ok(());
            }
        }
    }

    // Step 4: Import files
    let mut imported_count = 0;
    let mut failed_files = Vec::new();

    for (source_file, target_file) in &files_to_import {
        if dry_run {
            println!(
                "  {} {} -> {}",
                "Would import:".blue(),
                source_file.display().to_string().cyan(),
                target_file.display().to_string().green()
            );
        } else {
            match import_file(source_file, target_file) {
                Ok(_) => {
                    imported_count += 1;
                    if !track {
                        println!(
                            "  {} {}",
                            "Imported:".green(),
                            target_file.display().to_string().cyan()
                        );
                    }
                }
                Err(e) => {
                    super::print_error(&format!(
                        "Failed to import {}: {}",
                        source_file.display(),
                        e
                    ));
                    failed_files.push((source_file.clone(), e));
                }
            }
        }
    }

    if dry_run {
        super::print_info(&format!(
            "Dry run complete. Would import {} file{}",
            files_to_import.len(),
            if files_to_import.len() == 1 { "" } else { "s" }
        ));
        return Ok(());
    }

    // Step 5: Optionally track with dotman
    if track && imported_count > 0 {
        super::print_info("Tracking imported files with dotman...");

        let target_paths: Vec<String> = files_to_import
            .iter()
            .map(|(_, target)| target.display().to_string())
            .collect();

        // Use the add command to track files
        match crate::commands::add::execute(ctx, &target_paths, force) {
            Ok(_) => {
                super::print_success(&format!(
                    "Successfully tracked {} file{} with dotman",
                    imported_count,
                    if imported_count == 1 { "" } else { "s" }
                ));
            }
            Err(e) => {
                super::print_warning(&format!("Files imported but tracking failed: {}", e));
            }
        }
    }

    // Report results
    if !failed_files.is_empty() {
        super::print_warning(&format!(
            "Failed to import {} file{}",
            failed_files.len(),
            if failed_files.len() == 1 { "" } else { "s" }
        ));
    }

    if imported_count > 0 {
        super::print_success(&format!(
            "Successfully imported {} file{}{}",
            imported_count,
            if imported_count == 1 { "" } else { "s" },
            if track {
                " and tracked with dotman"
            } else {
                ""
            }
        ));
    }

    Ok(())
}

/// Clone a remote repository to a local directory
fn clone_repository(url: &str, target_dir: &Path) -> Result<()> {
    let output = Command::new("git")
        .args(["clone", "--depth", "1", url, target_dir.to_str().unwrap()])
        .output()
        .context("Failed to execute git clone")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to clone repository: {}", stderr);
    }

    Ok(())
}

/// Scan repository for all files to import
fn scan_repository(repo_path: &Path) -> Result<Vec<(PathBuf, PathBuf)>> {
    let home_dir =
        dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?;

    let mut files_to_import = Vec::new();

    // Walk through all files in the repository
    for entry in WalkDir::new(repo_path)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            // Skip .git directory and other hidden directories we don't want
            let name = e.file_name().to_string_lossy();
            if e.depth() == 1 && name == ".git" {
                return false;
            }
            true
        })
    {
        let entry = entry?;

        // Skip directories
        if entry.file_type().is_dir() {
            continue;
        }

        let source_path = entry.path().to_path_buf();

        // Calculate relative path from repo root
        let relative_path = source_path.strip_prefix(repo_path)?.to_path_buf();

        // Skip .git files
        if relative_path.components().any(|c| c.as_os_str() == ".git") {
            continue;
        }

        // Target path is home directory + relative path
        let target_path = home_dir.join(&relative_path);

        files_to_import.push((source_path, target_path));
    }

    // Sort for consistent output
    files_to_import.sort_by(|a, b| a.1.cmp(&b.1));

    Ok(files_to_import)
}

/// Check for existing files that would be overwritten
fn check_existing_files(files: &[(PathBuf, PathBuf)]) -> Result<Vec<(PathBuf, PathBuf)>> {
    let mut conflicts = Vec::new();

    for (source, target) in files {
        if target.exists() {
            conflicts.push((source.clone(), target.clone()));
        }
    }

    Ok(conflicts)
}

/// Import a single file from source to target
fn import_file(source: &Path, target: &Path) -> Result<()> {
    // Create parent directories if needed
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
    }

    // Copy the file
    fs::copy(source, target).with_context(|| {
        format!(
            "Failed to copy {} to {}",
            source.display(),
            target.display()
        )
    })?;

    // Preserve permissions on Unix systems
    #[cfg(unix)]
    {
        let metadata = fs::metadata(source)?;
        let permissions = metadata.permissions();
        fs::set_permissions(target, permissions)
            .with_context(|| format!("Failed to set permissions on {}", target.display()))?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_scan_repository() {
        // Create a temporary repository structure
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path();

        // Create some test files
        fs::create_dir_all(repo_path.join(".config/nvim")).unwrap();
        fs::write(repo_path.join(".bashrc"), "test bashrc").unwrap();
        fs::write(repo_path.join(".config/nvim/init.vim"), "test vim").unwrap();

        // Create .git directory that should be ignored
        fs::create_dir_all(repo_path.join(".git")).unwrap();
        fs::write(repo_path.join(".git/config"), "git config").unwrap();

        // Scan the repository
        let files = scan_repository(repo_path).unwrap();

        // Should find 2 files (excluding .git)
        assert_eq!(files.len(), 2);

        // Check that paths are correct
        let home_dir = dirs::home_dir().unwrap();
        assert!(
            files
                .iter()
                .any(|(_, target)| *target == home_dir.join(".bashrc"))
        );
        assert!(
            files
                .iter()
                .any(|(_, target)| *target == home_dir.join(".config/nvim/init.vim"))
        );
    }

    #[test]
    fn test_import_file() {
        let temp_dir = TempDir::new().unwrap();
        let source = temp_dir.path().join("source.txt");
        let target = temp_dir.path().join("subdir/target.txt");

        // Create source file
        fs::write(&source, "test content").unwrap();

        // Import the file
        import_file(&source, &target).unwrap();

        // Check that file was copied
        assert!(target.exists());
        let content = fs::read_to_string(&target).unwrap();
        assert_eq!(content, "test content");

        // Check that parent directory was created
        assert!(target.parent().unwrap().exists());
    }
}
