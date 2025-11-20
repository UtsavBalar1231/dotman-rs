use crate::DotmanContext;
use crate::mapping::MappingManager;
use crate::refs::RefManager;
use crate::storage::index::Index;
use anyhow::Result;

/// Execute fsck command - check repository consistency
///
/// Performs comprehensive consistency checks:
/// - Config/mapping consistency (orphaned remote references)
/// - Index/snapshot consistency (dangling references)
/// - Branch ref consistency (invalid commit IDs)
/// - Remote ref consistency (invalid mappings)
///
/// # Errors
///
/// Returns an error if:
/// - The repository is not initialized
/// - Cannot load index, mappings, or refs
pub fn execute(ctx: &DotmanContext) -> Result<()> {
    ctx.check_repo_initialized()?;

    super::print_info("Checking repository consistency...");

    let mut warnings = Vec::new();
    let mut errors = Vec::new();

    // Check 1: Config/Mapping Consistency
    super::print_info("Checking config/mapping consistency...");
    match check_config_mapping_consistency(ctx) {
        Ok(w) => warnings.extend(w),
        Err(e) => errors.push(format!("Config/mapping check failed: {e}")),
    }

    // Check 2: Branch Refs Consistency
    super::print_info("Checking branch refs...");
    match check_branch_refs(ctx) {
        Ok(w) => warnings.extend(w),
        Err(e) => errors.push(format!("Branch ref check failed: {e}")),
    }

    // Check 3: Remote Refs Consistency
    super::print_info("Checking remote refs...");
    match check_remote_refs(ctx) {
        Ok(w) => warnings.extend(w),
        Err(e) => errors.push(format!("Remote ref check failed: {e}")),
    }

    // Check 4: Index Consistency
    super::print_info("Checking index...");
    match check_index_consistency(ctx) {
        Ok(w) => warnings.extend(w),
        Err(e) => errors.push(format!("Index check failed: {e}")),
    }

    // Report results
    println!();
    if errors.is_empty() && warnings.is_empty() {
        super::print_success("Repository is consistent - no issues found");
    } else {
        if !errors.is_empty() {
            println!("Errors found:");
            for error in &errors {
                super::print_error(error);
            }
            println!();
        }

        if !warnings.is_empty() {
            println!("Warnings:");
            for warning in &warnings {
                super::print_warning(warning);
            }
            println!();
        }

        super::print_info(&format!(
            "Found {} error(s) and {} warning(s)",
            errors.len(),
            warnings.len()
        ));
    }

    Ok(())
}

/// Check config and mapping consistency
fn check_config_mapping_consistency(ctx: &DotmanContext) -> Result<Vec<String>> {
    let mapping_manager = MappingManager::new(&ctx.repo_path)?;
    mapping_manager.mapping().validate(&ctx.config)
}

/// Check branch refs point to valid commits
fn check_branch_refs(ctx: &DotmanContext) -> Result<Vec<String>> {
    let mut warnings = Vec::new();
    let ref_manager = RefManager::new(ctx.repo_path.clone());

    let branches = ref_manager.list_branches()?;
    for branch in branches {
        match ref_manager.get_branch_commit(&branch) {
            Ok(commit_id) => {
                // Check if commit exists
                let commit_path = ctx
                    .repo_path
                    .join("commits")
                    .join(format!("{commit_id}.zst"));
                if !commit_path.exists() {
                    warnings.push(format!(
                        "Branch '{}' points to non-existent commit '{}'",
                        branch,
                        &commit_id[..8.min(commit_id.len())]
                    ));
                }
            }
            Err(e) => {
                warnings.push(format!("Failed to read branch '{branch}': {e}"));
            }
        }
    }

    Ok(warnings)
}

/// Check remote refs consistency
fn check_remote_refs(ctx: &DotmanContext) -> Result<Vec<String>> {
    let mut warnings = Vec::new();
    let ref_manager = RefManager::new(ctx.repo_path.clone());

    // Check all configured remotes
    for remote_name in ctx.config.remotes.keys() {
        let refs = ref_manager.list_remote_refs(remote_name)?;
        for (branch, commit_id) in refs {
            // Check if commit exists
            let commit_path = ctx
                .repo_path
                .join("commits")
                .join(format!("{commit_id}.zst"));
            if !commit_path.exists() {
                warnings.push(format!(
                    "Remote ref '{}/{}' points to non-existent commit '{}'",
                    remote_name,
                    branch,
                    &commit_id[..8.min(commit_id.len())]
                ));
            }
        }
    }

    Ok(warnings)
}

/// Check index consistency
fn check_index_consistency(ctx: &DotmanContext) -> Result<Vec<String>> {
    let mut warnings = Vec::new();
    let index_path = ctx.repo_path.join(crate::INDEX_FILE);

    if !index_path.exists() {
        // No index yet - not an error
        return Ok(warnings);
    }

    let index = Index::load(&index_path)?;
    let objects_dir = ctx.repo_path.join("objects");

    // Check that all hashes in index have corresponding objects
    for (path, entry) in index.entries.iter().chain(index.staged_entries.iter()) {
        let object_path = objects_dir.join(format!("{}.zst", entry.hash));
        if !object_path.exists() {
            warnings.push(format!(
                "Index entry '{}' references missing object '{}'",
                path.display(),
                &entry.hash[..8.min(entry.hash.len())]
            ));
        }
    }

    Ok(warnings)
}
