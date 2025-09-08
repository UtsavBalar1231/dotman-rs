use crate::config::Config;
use crate::refs::RefManager;
use crate::storage::index::Index;
use crate::{DEFAULT_CONFIG_PATH, DEFAULT_REPO_DIR, INDEX_FILE};
use anyhow::{Context, Result};
use colored::Colorize;

/// Initialize a new dotman repository
///
/// # Errors
///
/// Returns an error if:
/// - Home directory cannot be found
/// - Repository already exists
/// - Failed to create repository directory structure
/// - Failed to initialize repository components
pub fn execute(bare: bool) -> Result<()> {
    let home = dirs::home_dir().context("Could not find home directory")?;
    let repo_path = home.join(DEFAULT_REPO_DIR);

    if repo_path.exists() && repo_path.join(INDEX_FILE).exists() {
        return Err(anyhow::anyhow!(
            "Dotman repository already initialized at {}",
            repo_path.display()
        ));
    }

    // Create repository structure
    std::fs::create_dir_all(&repo_path).with_context(|| {
        format!(
            "Failed to create repository directory: {}",
            repo_path.display()
        )
    })?;
    std::fs::create_dir_all(repo_path.join("commits"))
        .context("Failed to create commits directory")?;
    std::fs::create_dir_all(repo_path.join("objects"))
        .context("Failed to create objects directory")?;

    // Create empty index
    let index = Index::new();
    let index_path = repo_path.join(INDEX_FILE);
    index
        .save(&index_path)
        .context("Failed to save initial index")?;

    let ref_manager = RefManager::new(repo_path.clone());
    ref_manager
        .init()
        .context("Failed to initialize reference manager")?;

    // Create default config
    let config_path = home.join(DEFAULT_CONFIG_PATH);
    let config = Config::default();
    config
        .save(&config_path)
        .context("Failed to save default configuration")?;

    if bare {
        super::print_success(&format!(
            "Initialized bare dotman repository at {}",
            repo_path.display()
        ));
    } else {
        super::print_success(&format!(
            "Initialized dotman repository at {}",
            repo_path.display()
        ));
        println!("\n{}", "Quick start:".bold());
        println!("  dot add ~/.bashrc        # Track your bashrc");
        println!("  dot add ~/.config/nvim   # Track neovim config");
        println!("  dot status               # Check status");
        println!("  dot commit -m \"Initial\" # Create snapshot");
    }

    Ok(())
}
