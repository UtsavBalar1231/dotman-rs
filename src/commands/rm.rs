use crate::storage::index::Index;
use crate::{DotmanContext, INDEX_FILE};
use anyhow::Result;
use colored::Colorize;
use std::path::{Path, PathBuf};

pub fn execute(ctx: &DotmanContext, paths: &[String], cached: bool, force: bool) -> Result<()> {
    ctx.ensure_repo_exists()?;

    let index_path = ctx.repo_path.join(INDEX_FILE);
    let mut index = Index::load(&index_path)?;

    let mut removed_count = 0;
    let mut not_found_count = 0;

    for path_str in paths {
        let path = PathBuf::from(path_str);

        // Check if file is in index
        if index.get_entry(&path).is_none() && !force {
            super::print_warning(&format!("File not tracked: {}", path.display()));
            not_found_count += 1;
            continue;
        }

        // Remove from index
        if index.remove_entry(&path).is_some() {
            println!("  {} {}", "removed:".red(), path.display());
            removed_count += 1;
        }

        // Remove from filesystem if not --cached
        if !cached && path.exists() && (force || confirm_removal(&path)?) {
            std::fs::remove_file(&path)?;
            println!("  {} {}", "deleted:".red().bold(), path.display());
        }
    }

    // Save updated index
    if removed_count > 0 {
        index.save(&index_path)?;
        super::print_success(&format!("Removed {} file(s) from tracking", removed_count));
    }

    if not_found_count > 0 {
        super::print_info(&format!("{} file(s) were not tracked", not_found_count));
    }

    Ok(())
}

fn confirm_removal(path: &Path) -> Result<bool> {
    use std::io::{self, Write};

    print!("Remove file {} from filesystem? [y/N]: ", path.display());
    io::stdout().flush()?;

    let mut response = String::new();
    io::stdin().read_line(&mut response)?;

    Ok(response.trim().eq_ignore_ascii_case("y"))
}
