use std::path::PathBuf;
use tracing::info;
use anyhow::{Result, Context};

use crate::core::{
    types::{OperationMode, ProgressInfo},
    traits::{BackupEngine, RestoreEngine, ProgressReporter},
};
use crate::config::{Config, Profile, ProfileManager};
use crate::backup::{BackupManager, DefaultBackupEngine};
use crate::restore::{RestoreManager, DefaultRestoreEngine};
use crate::filesystem::FileSystemImpl;
use crate::cli::args::*;

/// Simple progress reporter for CLI
pub struct CliProgressReporter {
    verbose: bool,
}

impl CliProgressReporter {
    pub fn new(verbose: bool) -> Self {
        Self { verbose }
    }
}

#[async_trait::async_trait]
impl ProgressReporter for CliProgressReporter {
    fn report_progress(&self, progress: &ProgressInfo) {
        let percentage = if progress.total > 0 {
            (progress.current * 100) / progress.total
        } else {
            0
        };
        
        println!("[{}%] {}", percentage, progress.message);
        
        if let Some(details) = &progress.details {
            println!("    {}", details);
        }
    }
}

/// Main CLI command handler
pub struct CommandHandler {
    config: Config,
    profile_manager: ProfileManager,
    dry_run: bool,
    force: bool,
    verbose: bool,
}

impl CommandHandler {
    /// Create a new command handler
    pub async fn new(args: &DotmanArgs) -> Result<Self> {
        // Load configuration
        let config_path = args.config.clone().unwrap_or_else(|| {
            dirs::config_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("dotman")
                .join("config.toml")
        });

        let mut config = if config_path.exists() {
            Config::load(&config_path).await
                .context("Failed to load configuration")?
        } else {
            Config::default()
        };

        // Override config with command line arguments
        if args.dry_run {
            config.operation_mode = OperationMode::Preview;
        }

        if args.force {
            config.operation_mode = OperationMode::Force;
        } else if args.interactive {
            config.operation_mode = OperationMode::Interactive;
        }

        // Initialize profile manager
        let profile_manager = ProfileManager::new(config.config_dir.clone());

        Ok(Self {
            config,
            profile_manager,
            dry_run: args.dry_run,
            force: args.force,
            verbose: args.verbose > 0,
        })
    }

    /// Execute the main command
    pub async fn execute(&mut self, args: &DotmanArgs) -> Result<()> {
        match &args.command {
            Command::Init(init_args) => self.handle_init(init_args).await,
            Command::Backup(backup_args) => self.handle_backup(backup_args).await,
            Command::Restore(restore_args) => self.handle_restore(restore_args).await,
            Command::List(list_args) => self.handle_list(list_args).await,
            Command::Verify(verify_args) => self.handle_verify(verify_args).await,
            Command::Clean(clean_args) => self.handle_clean(clean_args).await,
            Command::Config(config_args) => self.handle_config(config_args).await,
            Command::Profile(profile_args) => self.handle_profile(profile_args).await,
            Command::Status(status_args) => self.handle_status(status_args).await,
            Command::Diff(diff_args) => self.handle_diff(diff_args).await,
        }
    }

    /// Handle init command
    async fn handle_init(&mut self, args: &InitArgs) -> Result<()> {
        info!("Initializing dotman configuration");

        let target_dir = args.target.clone().unwrap_or_else(|| {
            dirs::config_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("dotman")
        });

        // Create target directory
        tokio::fs::create_dir_all(&target_dir).await
            .context("Failed to create target directory")?;

        // Initialize configuration
        if args.defaults || !self.force {
            self.config = Config::default();
        }

        self.config.config_dir = target_dir.clone();
        
        if let Some(backup_dir) = &args.backup_dir {
            self.config.backup_dir = backup_dir.clone();
        }

        // Save configuration
        let config_path = target_dir.join("config.toml");
        self.config.save(&config_path).await
            .context("Failed to save configuration")?;

        // Initialize profile if specified
        if let Some(profile_name) = &args.profile {
            let profile = Profile::new(profile_name.clone(), self.config.clone());
            self.profile_manager.create_profile(profile_name.clone(), profile).await
                .context("Failed to create profile")?;
            
            self.profile_manager.set_active_profile(profile_name).await
                .context("Failed to set active profile")?;
        }

        println!("✓ Dotman initialized in {}", target_dir.display());
        if let Some(profile) = &args.profile {
            println!("✓ Created and activated profile: {}", profile);
        }

        Ok(())
    }

    /// Handle backup command
    async fn handle_backup(&mut self, args: &BackupArgs) -> Result<()> {
        info!("Starting backup operation");

        // Apply profile if specified
        if let Some(profile_name) = &args.profile {
            self.apply_profile(profile_name).await?;
        }

        // Update config with command line options
        if !args.exclude.is_empty() {
            self.config.exclude_patterns = args.exclude.clone();
        }
        if !args.include.is_empty() {
            self.config.include_patterns = args.include.clone();
        }
        self.config.verify_integrity = args.verify;

        // Create filesystem and progress reporter
        let filesystem = if self.dry_run {
            FileSystemImpl::new_dry_run()
        } else {
            FileSystemImpl::new()
        };
        let progress_reporter = CliProgressReporter::new(self.verbose);

        // Create backup manager
        let backup_manager = BackupManager::new(filesystem, progress_reporter, self.config.clone());

        // Perform backup
        let results = backup_manager.backup_files(args.paths.clone()).await
            .context("Backup operation failed")?;

        // Report results
        let successful = results.iter().filter(|r| r.success).count();
        let failed = results.iter().filter(|r| !r.success).count();

        println!("✓ Backup completed: {} successful, {} failed", successful, failed);

        if failed > 0 {
            println!("Failed operations:");
            for result in results.iter().filter(|r| !r.success) {
                println!("  ✗ {}: {}", 
                    result.path.display(), 
                    result.error.as_ref().unwrap_or(&"Unknown error".to_string())
                );
            }
        }

        Ok(())
    }

    /// Handle restore command
    async fn handle_restore(&mut self, args: &RestoreArgs) -> Result<()> {
        info!("Starting restore operation");

        // Apply profile if specified
        if let Some(profile_name) = &args.profile {
            self.apply_profile(profile_name).await?;
        }

        // Update config with command line options
        self.config.preserve_permissions = args.preserve_permissions;
        self.config.create_backups = args.backup_existing;

        // Resolve backup path
        let backup_path = self.resolve_backup_path(&args.backup).await?;

        // Determine target paths
        let target_paths = if args.files.is_empty() {
            if let Some(target) = &args.target {
                vec![target.clone()]
            } else if args.in_place {
                // TODO: Extract original paths from backup metadata
                vec![PathBuf::from("/")]
            } else {
                return Err(anyhow::anyhow!("No target specified for restore"));
            }
        } else {
            args.files.clone()
        };

        // Create filesystem and progress reporter
        let filesystem = if self.dry_run {
            FileSystemImpl::new_dry_run()
        } else {
            FileSystemImpl::new()
        };
        let progress_reporter = CliProgressReporter::new(self.verbose);

        // Create restore manager
        let restore_manager = RestoreManager::new(filesystem, progress_reporter, self.config.clone());

        // Perform restore
        let results = restore_manager.restore_files(backup_path, target_paths).await
            .context("Restore operation failed")?;

        // Report results
        let successful = results.iter().filter(|r| r.success).count();
        let failed = results.iter().filter(|r| !r.success).count();

        println!("✓ Restore completed: {} successful, {} failed", successful, failed);

        if failed > 0 {
            println!("Failed operations:");
            for result in results.iter().filter(|r| !r.success) {
                println!("  ✗ {}: {}", 
                    result.path.display(), 
                    result.error.as_ref().unwrap_or(&"Unknown error".to_string())
                );
            }
        }

        Ok(())
    }

    /// Handle list command
    async fn handle_list(&self, args: &ListArgs) -> Result<()> {
        match &args.target {
            ListTarget::Backups => {
                println!("Available backups:");
                // TODO: Implement backup listing
                println!("  (Not implemented yet)");
            }
            ListTarget::Contents { backup, detailed, filter: _ } => {
                let backup_path = self.resolve_backup_path(backup).await?;
                
                let filesystem = FileSystemImpl::new();
                let progress_reporter = CliProgressReporter::new(false);
                let restore_engine = DefaultRestoreEngine::new(filesystem, progress_reporter, self.config.clone());
                
                let contents = restore_engine.list_backup_contents(&backup_path).await
                    .context("Failed to list backup contents")?;

                println!("Backup contents for {}:", backup);
                for metadata in contents {
                    if *detailed {
                        println!("  {} ({:?}) - {} bytes", 
                            metadata.path.display(), 
                            metadata.file_type,
                            metadata.size
                        );
                    } else {
                        println!("  {}", metadata.path.display());
                    }
                }
            }
            ListTarget::Profiles => {
                let profiles = self.profile_manager.list_profiles();
                
                println!("Available profiles:");
                for profile_name in profiles {
                    let marker = if self.profile_manager.get_active_profile().map(|p| &p.name) == Some(&profile_name) {
                        "*"
                    } else {
                        " "
                    };
                    println!("{} {}", marker, profile_name);
                    if let Some(profile) = self.profile_manager.get_profile(&profile_name) {
                        if let Some(desc) = &profile.description {
                            println!("    {}", desc);
                        }
                    }
                }
            }
            ListTarget::Config => {
                println!("Configuration:");
                println!("  Config dir: {}", self.config.config_dir.display());
                println!("  Backup dir: {}", self.config.backup_dir.display());
                println!("  Operation mode: {:?}", self.config.operation_mode);
                println!("  Verify integrity: {}", self.config.verify_integrity);
                println!("  Preserve permissions: {}", self.config.preserve_permissions);
                println!("  Create backups: {}", self.config.create_backups);
            }
        }

        Ok(())
    }

    /// Handle verify command
    async fn handle_verify(&self, args: &VerifyArgs) -> Result<()> {
        let backup_path = self.resolve_backup_path(&args.backup).await?;
        
        let filesystem = FileSystemImpl::new();
        let progress_reporter = CliProgressReporter::new(self.verbose);
        let backup_engine = DefaultBackupEngine::new(filesystem, progress_reporter, self.config.clone());
        
        let is_valid = backup_engine.verify_backup(&backup_path).await
            .context("Verification failed")?;

        if is_valid {
            println!("✓ Backup is valid");
        } else {
            println!("✗ Backup verification failed");
            return Err(anyhow::anyhow!("Backup verification failed"));
        }

        Ok(())
    }

    /// Handle clean command
    async fn handle_clean(&self, _args: &CleanArgs) -> Result<()> {
        println!("Clean command not implemented yet");
        Ok(())
    }

    /// Handle config command
    async fn handle_config(&mut self, args: &ConfigArgs) -> Result<()> {
        match &args.action {
            ConfigAction::Show { key } => {
                if let Some(key) = key {
                    // TODO: Implement key-specific display
                    println!("Config key '{}' not found", key);
                } else {
                    println!("Current configuration:");
                    println!("{:#?}", self.config);
                }
            }
            ConfigAction::Set { key, value } => {
                // TODO: Implement config setting
                println!("Setting {} = {}", key, value);
            }
            ConfigAction::Get { key } => {
                // TODO: Implement config getting
                println!("Getting {}", key);
            }
            ConfigAction::Unset { key } => {
                // TODO: Implement config unsetting
                println!("Unsetting {}", key);
            }
            ConfigAction::Edit => {
                println!("Opening config editor not implemented yet");
            }
            ConfigAction::Validate => {
                match self.config.validate() {
                    Ok(_) => println!("✓ Configuration is valid"),
                    Err(e) => {
                        println!("✗ Configuration validation failed: {}", e);
                        return Err(e.into());
                    }
                }
            }
            ConfigAction::Reset { confirm } => {
                if *confirm || self.force {
                    self.config = Config::default();
                    println!("✓ Configuration reset to defaults");
                } else {
                    println!("Use --confirm to reset configuration");
                }
            }
        }

        Ok(())
    }

    /// Handle profile command
    async fn handle_profile(&mut self, args: &ProfileArgs) -> Result<()> {
        match &args.action {
            ProfileAction::List => {
                let profiles = self.profile_manager.list_profiles();
                
                for profile_name in profiles {
                    let marker = if self.profile_manager.get_active_profile().map(|p| &p.name) == Some(&profile_name) {
                        "*"
                    } else {
                        " "
                    };
                    println!("{} {}", marker, profile_name);
                }
            }
            ProfileAction::Create { name, description, from } => {
                let profile = if let Some(from_name) = from {
                    if let Some(source_profile) = self.profile_manager.get_profile(from_name) {
                        let mut new_profile = source_profile.clone();
                        new_profile.name = name.clone();
                        new_profile.description = description.clone();
                        new_profile
                    } else {
                        return Err(anyhow::anyhow!("Source profile '{}' not found", from_name));
                    }
                } else {
                    Profile::new(name.clone(), self.config.clone()).with_description(description.clone().unwrap_or_default())
                };
                
                self.profile_manager.create_profile(name.clone(), profile).await
                    .context("Failed to create profile")?;
                
                println!("✓ Created profile: {}", name);
            }
            ProfileAction::Delete { name, force } => {
                if *force || self.force || self.confirm(&format!("Delete profile '{}'?", name)).await? {
                    self.profile_manager.remove_profile(name).await
                        .context("Failed to delete profile")?;
                    println!("✓ Deleted profile: {}", name);
                } else {
                    println!("Profile deletion cancelled");
                }
            }
            ProfileAction::Switch { name } => {
                self.profile_manager.set_active_profile(name).await
                    .context("Failed to switch profile")?;
                println!("✓ Switched to profile: {}", name);
            }
            ProfileAction::Current => {
                if let Some(active) = self.profile_manager.get_active_profile() {
                    println!("Current profile: {}", active.name);
                } else {
                    println!("No active profile");
                }
            }
            ProfileAction::Edit { name: _ } => {
                println!("Profile editing not implemented yet");
            }
            ProfileAction::Rename { old_name, new_name } => {
                self.profile_manager.rename_profile(old_name, new_name).await
                    .context("Failed to rename profile")?;
                println!("✓ Renamed profile: {} -> {}", old_name, new_name);
            }
        }

        Ok(())
    }

    /// Handle status command
    async fn handle_status(&self, _args: &StatusArgs) -> Result<()> {
        println!("Status command not implemented yet");
        Ok(())
    }

    /// Handle diff command
    async fn handle_diff(&self, _args: &DiffArgs) -> Result<()> {
        println!("Diff command not implemented yet");
        Ok(())
    }

    /// Apply a profile to current configuration
    async fn apply_profile(&mut self, profile_name: &str) -> Result<()> {
        let profile = self.profile_manager.get_profile(profile_name)
            .ok_or_else(|| anyhow::anyhow!("Profile '{}' not found", profile_name))?;

        // Apply profile overrides to config
        self.config = profile.get_effective_config(None);
        
        Ok(())
    }

    /// Resolve backup path from name or path
    async fn resolve_backup_path(&self, backup: &str) -> Result<PathBuf> {
        let path = PathBuf::from(backup);
        
        if path.is_absolute() && path.exists() {
            Ok(path)
        } else {
            // Try to resolve as backup name in backup directory
            let backup_path = self.config.backup_dir.join(backup);
            if backup_path.exists() {
                Ok(backup_path)
            } else {
                Err(anyhow::anyhow!("Backup '{}' not found", backup))
            }
        }
    }

    /// Ask user for confirmation
    async fn confirm(&self, message: &str) -> Result<bool> {
        if self.force {
            return Ok(true);
        }

        print!("{} [y/N]: ", message);
        use std::io::{self, Write};
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        
        Ok(input.trim().to_lowercase() == "y" || input.trim().to_lowercase() == "yes")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_command_handler_creation() {
        let temp_dir = TempDir::new().unwrap();
        
        let args = DotmanArgs {
            verbose: 1,
            config: Some(temp_dir.path().join("config.toml")),
            dry_run: true,
            force: false,
            interactive: false,
            directory: None,
            command: Command::Init(InitArgs {
                target: None,
                defaults: true,
                backup_dir: None,
                profile: None,
            }),
        };

        let result = CommandHandler::new(&args).await;
        assert!(result.is_ok());
    }
} 