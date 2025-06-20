use std::path::PathBuf;
use tracing::info;
use anyhow::{Result, Context};

use crate::core::{
    types::{OperationMode, ProgressInfo},
    traits::{BackupEngine, RestoreEngine, ProgressReporter, FileSystem},
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
    filesystem: FileSystemImpl,
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

        // Initialize filesystem
        let filesystem = if args.dry_run {
            FileSystemImpl::new_dry_run()
        } else {
            FileSystemImpl::new()
        };

        Ok(Self {
            config,
            profile_manager,
            dry_run: args.dry_run,
            force: args.force,
            verbose: args.verbose > 0,
            filesystem,
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
                let backup_root = &self.config.backup_dir;
                
                if !self.filesystem.exists(backup_root).await? {
                    println!("No backup directory found at: {}", backup_root.display());
                    return Ok(());
                }

                // Find all backup directories
                let mut backups = Vec::new();
                let entries = self.filesystem.list_dir(backup_root).await?;
                
                for entry in entries {
                    let entry_path = backup_root.join(&entry);
                    let metadata = self.filesystem.metadata(&entry_path).await?;
                    
                    if metadata.is_directory() {
                        if let Some(name) = entry_path.file_name().and_then(|n| n.to_str()) {
                            if name.starts_with("backup-") {
                                // Try to load session metadata
                                let metadata_path = entry_path.join("session_metadata.json");
                                let (session_info, total_files, total_size_mb) = if self.filesystem.exists(&metadata_path).await.unwrap_or(false) {
                                    // Try to read and parse session metadata
                                    match self.filesystem.read_file(&metadata_path).await {
                                        Ok(content) => {
                                            match serde_json::from_slice::<serde_json::Value>(&content) {
                                                Ok(json) => {
                                                    let started_at = json.get("started_at")
                                                        .and_then(|v| v.as_str())
                                                        .unwrap_or("Unknown");
                                                    let processed_files = json.get("processed_files")
                                                        .and_then(|v| v.as_u64())
                                                        .unwrap_or(0);
                                                    let processed_size = json.get("processed_size")
                                                        .and_then(|v| v.as_u64())
                                                        .unwrap_or(0);
                                                    let size_mb = if processed_size > 0 {
                                                        processed_size / (1024 * 1024)
                                                    } else {
                                                        // Fallback: calculate actual directory size
                                                        self.calculate_directory_size(&entry_path).await.unwrap_or(0) / (1024 * 1024)
                                                    };
                                                    
                                                    (started_at.to_string(), processed_files, size_mb)
                                                },
                                                Err(_) => ("Metadata parse error".to_string(), 0, 0),
                                            }
                                        },
                                        Err(_) => ("Metadata read error".to_string(), 0, 0),
                                    }
                                } else {
                                    // No session metadata, calculate manually
                                    let actual_size = self.calculate_directory_size(&entry_path).await.unwrap_or(0);
                                    let actual_files = self.count_files_in_directory(&entry_path).await.unwrap_or(0);
                                    let size_mb = actual_size / (1024 * 1024);
                                    (format!("Created: {}", metadata.modified.format("%Y-%m-%d %H:%M:%S")), actual_files, size_mb)
                                };

                                backups.push((metadata.modified, entry_path.clone(), session_info, total_files, total_size_mb));
                            }
                        }
                    }
                }

                if backups.is_empty() {
                    println!("No backups found");
                    return Ok(());
                }

                // Sort by modification time (newest first)
                backups.sort_by(|a, b| b.0.cmp(&a.0));

                println!("Available backups:");
                for (_, path, info, files, size_mb) in backups {
                    let name = path.file_name()
                        .unwrap_or_default()
                        .to_string_lossy();
                    println!("  {} ({} MB) - {} files, {}", name, size_mb, files, info);
                }
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
                        let type_str = match metadata.file_type {
                            crate::core::types::FileType::File => "File",
                            crate::core::types::FileType::Directory => "Dir",
                            crate::core::types::FileType::Symlink { .. } => "Link",
                            _ => "Other",
                        };
                        println!("  {} [{}] {} bytes - {}", 
                            metadata.path.display(), 
                            type_str,
                            metadata.size,
                            metadata.modified.format("%Y-%m-%d %H:%M:%S")
                        );
                    } else {
                        println!("  {}", metadata.path.display());
                    }
                }
            }
            ListTarget::Profiles => {
                let profiles = self.profile_manager.list_profiles();
                
                if profiles.is_empty() {
                    println!("No profiles found");
                    return Ok(());
                }

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
                println!("  Follow symlinks: {}", self.config.follow_symlinks);
                println!("  Max backup versions: {}", self.config.max_backup_versions);
                println!("  Log level: {}", self.config.log_level);
                if !self.config.include_patterns.is_empty() {
                    println!("  Include patterns: {:?}", self.config.include_patterns);
                }
                if !self.config.exclude_patterns.is_empty() {
                    println!("  Exclude patterns: {:?}", self.config.exclude_patterns);
                }
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
    async fn handle_clean(&self, args: &CleanArgs) -> Result<()> {
        info!("Starting backup cleanup");

        let backup_root = &self.config.backup_dir;
        
        if !self.filesystem.exists(backup_root).await? {
            println!("No backup directory found at: {}", backup_root.display());
            return Ok(());
        }

        // Find all backup directories
        let mut backups = Vec::new();
        let entries = self.filesystem.list_dir(backup_root).await?;
        
        for entry in entries {
            let entry_path = backup_root.join(&entry);
            let metadata = self.filesystem.metadata(&entry_path).await?;
            
            if metadata.is_directory() {
                if let Some(name) = entry_path.file_name().and_then(|n| n.to_str()) {
                    if name.starts_with("backup-") {
                        backups.push((metadata.modified, entry_path, metadata.size));
                    }
                }
            }
        }

        if backups.is_empty() {
            println!("No backups found to clean");
            return Ok(());
        }

        // Sort by modification time (oldest first)
        backups.sort_by_key(|(modified, _, _)| *modified);

        let mut cleaned_count = 0;
        let mut total_space_freed = 0u64;

        if let Some(max_age_days) = args.older_than_days {
            // Clean backups older than specified days
            let cutoff_time = chrono::Utc::now() - chrono::Duration::days(max_age_days as i64);
            
            for (modified, path, size) in &backups {
                if *modified < cutoff_time {
                    if args.force || self.force || self.confirm(&format!("Remove backup '{}'?", path.display())).await? {
                        println!("Removing backup: {}", path.display());
                        if !self.dry_run {
                            if let Err(e) = self.filesystem.remove(path).await {
                                eprintln!("Failed to remove {}: {}", path.display(), e);
                            } else {
                                cleaned_count += 1;
                                total_space_freed += size;
                            }
                        } else {
                            cleaned_count += 1;
                            total_space_freed += size;
                        }
                    }
                }
            }
        } else if let Some(keep_count) = args.keep_last {
            // Keep only the last N backups
            let to_remove = if backups.len() > keep_count {
                backups.len() - keep_count
            } else {
                0
            };

            for (_, path, size) in backups.iter().take(to_remove) {
                if args.force || self.force || self.confirm(&format!("Remove backup '{}'?", path.display())).await? {
                    println!("Removing backup: {}", path.display());
                    if !self.dry_run {
                        if let Err(e) = self.filesystem.remove(path).await {
                            eprintln!("Failed to remove {}: {}", path.display(), e);
                        } else {
                            cleaned_count += 1;
                            total_space_freed += size;
                        }
                    } else {
                        cleaned_count += 1;
                        total_space_freed += size;
                    }
                }
            }
        } else {
            // Interactive mode - show all backups and let user choose
            println!("Available backups (oldest first):");
            for (i, (modified, path, size)) in backups.iter().enumerate() {
                let size_mb = size / (1024 * 1024);
                println!("  {}: {} ({} MB) - {}", 
                    i + 1, 
                    path.file_name().unwrap_or_default().to_string_lossy(),
                    size_mb,
                    modified.format("%Y-%m-%d %H:%M:%S")
                );
            }

            if self.confirm("Remove all old backups (keeping most recent)?").await? {
                // Keep only the newest backup
                for (_, path, size) in backups.iter().take(backups.len().saturating_sub(1)) {
                    println!("Removing backup: {}", path.display());
                    if !self.dry_run {
                        if let Err(e) = self.filesystem.remove(path).await {
                            eprintln!("Failed to remove {}: {}", path.display(), e);
                        } else {
                            cleaned_count += 1;
                            total_space_freed += size;
                        }
                    } else {
                        cleaned_count += 1;
                        total_space_freed += size;
                    }
                }
            }
        }

        let space_mb = total_space_freed / (1024 * 1024);
        if self.dry_run {
            println!("✓ Dry run: Would clean {} backups, freeing {} MB", cleaned_count, space_mb);
        } else {
            println!("✓ Cleaned {} backups, freed {} MB", cleaned_count, space_mb);
        }

        Ok(())
    }

    /// Handle config command
    async fn handle_config(&mut self, args: &ConfigArgs) -> Result<()> {
        match &args.action {
            ConfigAction::Show { key } => {
                if let Some(key) = key {
                    match key.as_str() {
                        "backup_dir" => println!("{}", self.config.backup_dir.display()),
                        "config_dir" => println!("{}", self.config.config_dir.display()),
                        "max_backup_versions" => println!("{}", self.config.max_backup_versions),
                        "log_level" => println!("{}", self.config.log_level),
                        "verify_integrity" => println!("{}", self.config.verify_integrity),
                        "preserve_permissions" => println!("{}", self.config.preserve_permissions),
                        "create_backups" => println!("{}", self.config.create_backups),
                        "follow_symlinks" => println!("{}", self.config.follow_symlinks),
                        "operation_mode" => println!("{:?}", self.config.operation_mode),
                        "include_patterns" => {
                            for pattern in &self.config.include_patterns {
                                println!("{}", pattern);
                            }
                        },
                        "exclude_patterns" => {
                            for pattern in &self.config.exclude_patterns {
                                println!("{}", pattern);
                            }
                        },
                        _ => println!("Unknown configuration key: {}", key),
                    }
                } else {
                    println!("Current configuration:");
                    println!("  backup_dir = {}", self.config.backup_dir.display());
                    println!("  config_dir = {}", self.config.config_dir.display());
                    println!("  max_backup_versions = {}", self.config.max_backup_versions);
                    println!("  log_level = {}", self.config.log_level);
                    println!("  verify_integrity = {}", self.config.verify_integrity);
                    println!("  preserve_permissions = {}", self.config.preserve_permissions);
                    println!("  create_backups = {}", self.config.create_backups);
                    println!("  follow_symlinks = {}", self.config.follow_symlinks);
                    println!("  operation_mode = {:?}", self.config.operation_mode);
                    println!("  include_patterns = {:?}", self.config.include_patterns);
                    println!("  exclude_patterns = {:?}", self.config.exclude_patterns);
                }
            }
            ConfigAction::Set { key, value } => {
                match key.as_str() {
                    "backup_dir" => {
                        self.config.backup_dir = PathBuf::from(value);
                        println!("✓ Set backup_dir = {}", value);
                    },
                    "config_dir" => {
                        self.config.config_dir = PathBuf::from(value);
                        println!("✓ Set config_dir = {}", value);
                    },
                    "max_backup_versions" => {
                        let versions: u32 = value.parse()
                            .context("Invalid number for max_backup_versions")?;
                        self.config.max_backup_versions = versions;
                        println!("✓ Set max_backup_versions = {}", versions);
                    },
                    "log_level" => {
                        self.config.log_level = value.clone();
                        println!("✓ Set log_level = {}", value);
                    },
                    "verify_integrity" => {
                        let verify: bool = value.parse()
                            .context("Invalid boolean for verify_integrity (use true/false)")?;
                        self.config.verify_integrity = verify;
                        println!("✓ Set verify_integrity = {}", verify);
                    },
                    "preserve_permissions" => {
                        let preserve: bool = value.parse()
                            .context("Invalid boolean for preserve_permissions (use true/false)")?;
                        self.config.preserve_permissions = preserve;
                        println!("✓ Set preserve_permissions = {}", preserve);
                    },
                    "create_backups" => {
                        let create: bool = value.parse()
                            .context("Invalid boolean for create_backups (use true/false)")?;
                        self.config.create_backups = create;
                        println!("✓ Set create_backups = {}", create);
                    },
                    "follow_symlinks" => {
                        let follow: bool = value.parse()
                            .context("Invalid boolean for follow_symlinks (use true/false)")?;
                        self.config.follow_symlinks = follow;
                        println!("✓ Set follow_symlinks = {}", follow);
                    },
                    _ => {
                        return Err(anyhow::anyhow!("Unknown configuration key: {}", key));
                    }
                }

                // Save updated configuration
                let config_path = self.config.config_dir.join("config.toml");
                self.config.save(&config_path).await
                    .context("Failed to save updated configuration")?;
            }
            ConfigAction::Get { key } => {
                match key.as_str() {
                    "backup_dir" => println!("{}", self.config.backup_dir.display()),
                    "config_dir" => println!("{}", self.config.config_dir.display()),
                    "max_backup_versions" => println!("{}", self.config.max_backup_versions),
                    "log_level" => println!("{}", self.config.log_level),
                    "verify_integrity" => println!("{}", self.config.verify_integrity),
                    "preserve_permissions" => println!("{}", self.config.preserve_permissions),
                    "create_backups" => println!("{}", self.config.create_backups),
                    "follow_symlinks" => println!("{}", self.config.follow_symlinks),
                    "operation_mode" => println!("{:?}", self.config.operation_mode),
                    _ => return Err(anyhow::anyhow!("Unknown configuration key: {}", key)),
                }
            }
            ConfigAction::Unset { key } => {
                match key.as_str() {
                    "include_patterns" => {
                        self.config.include_patterns.clear();
                        println!("✓ Cleared include_patterns");
                    },
                    "exclude_patterns" => {
                        self.config.exclude_patterns.clear();
                        println!("✓ Cleared exclude_patterns");
                    },
                    _ => {
                        return Err(anyhow::anyhow!("Cannot unset key '{}' (only patterns can be cleared)", key));
                    }
                }

                // Save updated configuration
                let config_path = self.config.config_dir.join("config.toml");
                self.config.save(&config_path).await
                    .context("Failed to save updated configuration")?;
            }
            ConfigAction::Edit => {
                let config_path = self.config.config_dir.join("config.toml");
                let editor = std::env::var("EDITOR").unwrap_or_else(|_| "nano".to_string());
                
                println!("Opening {} with {}", config_path.display(), editor);
                
                let status = std::process::Command::new(&editor)
                    .arg(&config_path)
                    .status()
                    .context("Failed to start editor")?;

                if !status.success() {
                    return Err(anyhow::anyhow!("Editor exited with non-zero status"));
                }

                // Reload configuration after editing
                self.config = Config::load(&config_path).await
                    .context("Failed to reload configuration after editing")?;

                println!("✓ Configuration reloaded");
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
                if *confirm || self.force || self.confirm("Reset configuration to defaults?").await? {
                    self.config = Config::default();
                    
                    // Save reset configuration
                    let config_path = self.config.config_dir.join("config.toml");
                    self.config.save(&config_path).await
                        .context("Failed to save reset configuration")?;
                    
                    println!("✓ Configuration reset to defaults");
                } else {
                    println!("Configuration reset cancelled");
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
        println!("Dotman Status Report");
        println!("===================");

        // Configuration status
        println!("\nConfiguration:");
        println!("  Config directory: {}", self.config.config_dir.display());
        println!("  Backup directory: {}", self.config.backup_dir.display());
        
        // Check if directories exist
        let config_exists = self.filesystem.exists(&self.config.config_dir).await?;
        let backup_exists = self.filesystem.exists(&self.config.backup_dir).await?;
        
        println!("  Config dir exists: {}", if config_exists { "✓" } else { "✗" });
        println!("  Backup dir exists: {}", if backup_exists { "✓" } else { "✗" });

        // Profile status
        println!("\nProfiles:");
        let profiles = self.profile_manager.list_profiles();
        if profiles.is_empty() {
            println!("  No profiles configured");
        } else {
            println!("  Total profiles: {}", profiles.len());
            if let Some(active) = self.profile_manager.get_active_profile() {
                println!("  Active profile: {}", active.name);
            } else {
                println!("  Active profile: None");
            }
        }

        // Backup status
        println!("\nBackups:");
        if backup_exists {
            let mut backup_count = 0;
            let mut total_size = 0u64;
            
            let entries = self.filesystem.list_dir(&self.config.backup_dir).await?;
            for entry in entries {
                let entry_path = self.config.backup_dir.join(&entry);
                let metadata = self.filesystem.metadata(&entry_path).await?;
                
                if metadata.is_directory() {
                    if let Some(name) = entry_path.file_name().and_then(|n| n.to_str()) {
                        if name.starts_with("backup-") {
                            backup_count += 1;
                            total_size += metadata.size;
                        }
                    }
                }
            }

            println!("  Total backups: {}", backup_count);
            let size_mb = total_size / (1024 * 1024);
            println!("  Total backup size: {} MB", size_mb);

            if backup_count > 0 {
                // Find newest backup
                let mut newest_backup: Option<(chrono::DateTime<chrono::Utc>, PathBuf)> = None;
                let entries = self.filesystem.list_dir(&self.config.backup_dir).await?;
                
                for entry in entries {
                    let entry_path = self.config.backup_dir.join(&entry);
                    let metadata = self.filesystem.metadata(&entry_path).await?;
                    
                    if metadata.is_directory() {
                        if let Some(name) = entry_path.file_name().and_then(|n| n.to_str()) {
                            if name.starts_with("backup-") {
                                if newest_backup.is_none() || newest_backup.as_ref().unwrap().0 < metadata.modified {
                                    newest_backup = Some((metadata.modified, entry_path));
                                }
                            }
                        }
                    }
                }

                if let Some((modified, path)) = newest_backup {
                    println!("  Latest backup: {} ({})", 
                        path.file_name().unwrap_or_default().to_string_lossy(),
                        modified.format("%Y-%m-%d %H:%M:%S")
                    );
                }
            }
        } else {
            println!("  Backup directory not found");
        }

        // Configuration validation
        println!("\nConfiguration Validation:");
        match self.config.validate() {
            Ok(_) => println!("  ✓ Configuration is valid"),
            Err(e) => println!("  ✗ Configuration error: {}", e),
        }

        // System information
        println!("\nSystem Information:");
        println!("  Operation mode: {:?}", self.config.operation_mode);
        println!("  Dry run mode: {}", self.dry_run);
        println!("  Force mode: {}", self.force);
        println!("  Verbose mode: {}", self.verbose);

        Ok(())
    }

    /// Handle diff command
    async fn handle_diff(&self, args: &DiffArgs) -> Result<()> {
        let backup_path = self.resolve_backup_path(&args.backup).await?;
        
        println!("Comparing backup '{}' with current files", args.backup);
        println!("==========================================");

        // Get backup contents
        let filesystem = FileSystemImpl::new();
        let progress_reporter = CliProgressReporter::new(false);
        let restore_engine = DefaultRestoreEngine::new(filesystem, progress_reporter, self.config.clone());
        
        let backup_contents = restore_engine.list_backup_contents(&backup_path).await
            .context("Failed to list backup contents")?;

        let mut differences_found = false;

        for backup_metadata in &backup_contents {
            // Calculate expected current path (assuming backup path structure)
            let relative_path = backup_metadata.path.strip_prefix(&backup_path)
                .map_err(|_| anyhow::anyhow!("Invalid backup path structure"))?;
            let current_path = if args.files.is_empty() {
                PathBuf::from("/").join(relative_path)
            } else {
                // Check if this file is in the specified files list
                let mut found = false;
                let mut target_path = PathBuf::new();
                for file in &args.files {
                    if backup_metadata.path.ends_with(file) {
                        target_path = file.clone();
                        found = true;
                        break;
                    }
                }
                if !found {
                    continue;
                }
                target_path
            };

            // Check if current file exists
            if !self.filesystem.exists(&current_path).await? {
                differences_found = true;
                println!("✗ MISSING: {} (exists in backup, not in current)", current_path.display());
                continue;
            }

            // Compare metadata
            let current_metadata = self.filesystem.metadata(&current_path).await?;
            
            if backup_metadata.size != current_metadata.size {
                differences_found = true;
                println!("△ SIZE: {} (backup: {} bytes, current: {} bytes)", 
                    current_path.display(),
                    backup_metadata.size,
                    current_metadata.size
                );
            }

            if backup_metadata.modified != current_metadata.modified {
                differences_found = true;
                if args.show_timestamps {
                    println!("△ TIME: {} (backup: {}, current: {})", 
                        current_path.display(),
                        backup_metadata.modified.format("%Y-%m-%d %H:%M:%S"),
                        current_metadata.modified.format("%Y-%m-%d %H:%M:%S")
                    );
                } else {
                    println!("△ TIME: {} (modified since backup)", current_path.display());
                }
            }

            // Compare file type
            let backup_type = format!("{:?}", backup_metadata.file_type);
            let current_type = format!("{:?}", current_metadata.file_type);
            if backup_type != current_type {
                differences_found = true;
                println!("△ TYPE: {} (backup: {}, current: {})", 
                    current_path.display(),
                    backup_type,
                    current_type
                );
            }

            // For identical files, optionally show
            if args.show_identical && 
               backup_metadata.size == current_metadata.size &&
               backup_metadata.modified == current_metadata.modified {
                println!("✓ SAME: {}", current_path.display());
            }
        }

        // Check for files that exist currently but not in backup
        if !args.files.is_empty() {
            for file in &args.files {
                if self.filesystem.exists(file).await? {
                    // Check if this file is represented in the backup
                    let file_in_backup = backup_contents.iter()
                        .any(|meta| meta.path.ends_with(file));
                    
                    if !file_in_backup {
                        differences_found = true;
                        println!("+ NEW: {} (exists currently, not in backup)", file.display());
                    }
                }
            }
        }

        if !differences_found {
            println!("✓ No differences found between backup and current files");
        }

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
        if !self.force {
            print!("{} (y/N): ", message);
            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;
            Ok(input.trim().to_lowercase() == "y")
        } else {
            Ok(true)
        }
    }

    /// Calculate the total size of all files in a directory recursively
    fn calculate_directory_size<'a>(&'a self, dir_path: &'a std::path::Path) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<u64>> + Send + 'a>> {
        Box::pin(async move {
        let mut total_size = 0;
        
        if !self.filesystem.exists(dir_path).await? {
            return Ok(0);
        }

        if let Ok(entries) = self.filesystem.list_dir(dir_path).await {
            for entry in entries {
                let entry_path = dir_path.join(&entry);
                if let Ok(metadata) = self.filesystem.metadata(&entry_path).await {
                    match metadata.file_type {
                        crate::core::types::FileType::File => {
                            total_size += metadata.size;
                        }
                        crate::core::types::FileType::Directory => {
                            total_size += self.calculate_directory_size(&entry_path).await.unwrap_or(0);
                        }
                        _ => {
                            // For symlinks and other types, add their metadata size
                            total_size += metadata.size;
                        }
                    }
                }
            }
        }

        Ok(total_size)
        })
    }

    /// Count the total number of files in a directory recursively
    fn count_files_in_directory<'a>(&'a self, dir_path: &'a std::path::Path) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<u64>> + Send + 'a>> {
        Box::pin(async move {
        let mut total_files = 0;
        
        if !self.filesystem.exists(dir_path).await? {
            return Ok(0);
        }

        if let Ok(entries) = self.filesystem.list_dir(dir_path).await {
            for entry in entries {
                let entry_path = dir_path.join(&entry);
                if let Ok(metadata) = self.filesystem.metadata(&entry_path).await {
                    match metadata.file_type {
                        crate::core::types::FileType::File => {
                            total_files += 1;
                        }
                        crate::core::types::FileType::Directory => {
                            total_files += self.count_files_in_directory(&entry_path).await.unwrap_or(0);
                        }
                        crate::core::types::FileType::Symlink { .. } => {
                            total_files += 1;
                        }
                        _ => {
                            // Other file types
                        }
                    }
                }
            }
        }

        Ok(total_files)
        })
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