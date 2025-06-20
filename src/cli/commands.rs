use std::path::{Path, PathBuf};
use anyhow::Context;
use tracing::info;

use crate::cli::args::*;
use crate::backup::{BackupManager, BackupSession};
use crate::restore::manager::RestoreManager;
use crate::config::{Config, profile::Profile, ProfileManager};
use crate::config::config::PackageConfig;
use crate::core::{
    types::{OperationMode, ProgressInfo, OperationType, OperationResult},
    traits::{BackupEngine, RestoreEngine, ProgressReporter, FileSystem},
};
use crate::{FileSystemImpl};
use crate::backup::engine::DefaultBackupEngine;
use crate::restore::engine::DefaultRestoreEngine;

/// File status for change detection
#[derive(Debug, Clone, PartialEq)]
pub enum FileStatus {
    Unchanged,
    Modified,
    New,
    Missing,
}

/// Simple progress reporter for CLI
#[derive(Clone)]
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
    pub async fn new(args: &DotmanArgs) -> anyhow::Result<Self> {
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
    pub async fn execute(&mut self, args: &DotmanArgs) -> anyhow::Result<()> {
        match &args.command {
            Command::Init(init_args) => self.handle_init(init_args).await,
            Command::Backup(backup_args) => self.handle_backup(backup_args).await,
            Command::Restore(restore_args) => self.handle_restore(restore_args).await,
            Command::List(list_args) => self.handle_list(list_args).await,
            Command::Verify(verify_args) => self.handle_verify(verify_args).await,
            Command::Clean(clean_args) => self.handle_clean(clean_args).await,
            Command::Config(config_args) => self.handle_config(config_args).await,
            Command::Profile(profile_args) => self.handle_profile(profile_args).await,
            Command::Package(package_args) => self.handle_package(package_args).await,
            Command::Status(status_args) => self.handle_status(status_args).await,
            Command::Diff(diff_args) => self.handle_diff(diff_args).await,
        }
    }

    /// Handle init command
    async fn handle_init(&mut self, args: &InitArgs) -> anyhow::Result<()> {
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

        println!("[✓] Dotman initialized in {}", target_dir.display());
        if let Some(profile) = &args.profile {
            println!("[✓] Created and activated profile: {}", profile);
        }

        Ok(())
    }

    /// Handle backup command
    async fn handle_backup(&mut self, args: &BackupArgs) -> anyhow::Result<()> {
        info!("Starting backup operation");

        // Apply profile if specified
        if let Some(profile_name) = &args.profile {
            self.apply_profile(profile_name).await?;
        }

        // Determine backup paths and metadata
        let (backup_paths, package_name, package_description, backup_name, backup_description) = if let Some(package_name) = &args.package {
            // Package-based backup
            if let Some(package_config) = self.config.get_package(package_name) {
                // Clone all needed values to avoid borrowing issues
                let paths = package_config.paths.clone();
                let pkg_description = package_config.description.clone();
                let package_exclude = package_config.exclude_patterns.clone();
                let package_include = package_config.include_patterns.clone();
                
                let name = args.name.clone().unwrap_or_else(|| format!("package-{}", package_name));
                let description = args.description.clone().unwrap_or_else(|| 
                    format!("{} - {}", pkg_description, chrono::Utc::now().format("%Y-%m-%d %H:%M:%S"))
                );
                
                // Apply package-specific patterns if any
                if !package_exclude.is_empty() {
                    self.config.exclude_patterns.extend(package_exclude);
                }
                if !package_include.is_empty() {
                    self.config.include_patterns.extend(package_include);
                }
                
                println!("[📦] Using package configuration: {}", package_name);
                println!("[📦] Package description: {}", pkg_description);
                println!("[📦] Backing up {} paths", paths.len());
                
                (paths, Some(package_name.clone()), Some(pkg_description), Some(name), Some(description))
            } else {
                return Err(anyhow::anyhow!("Package '{}' not found in configuration. Available packages: {:?}", 
                    package_name, self.config.list_packages()));
            }
        } else {
            // Regular path-based backup
            if args.paths.is_empty() {
                return Err(anyhow::anyhow!("No paths specified for backup. Use --package <name> for package backup or specify paths directly."));
            }
            (args.paths.clone(), None, None, args.name.clone(), args.description.clone())
        };

        // Update config with command line options
        if !args.exclude.is_empty() {
            self.config.exclude_patterns.extend(args.exclude.clone());
        }
        if !args.include.is_empty() {
            self.config.include_patterns.extend(args.include.clone());
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

        // Perform backup with metadata
        let session = backup_manager.start_backup_session_with_metadata(
            backup_paths,
            package_name,
            package_description,
            backup_name.clone(),
            backup_description.clone(),
        ).await.context("Failed to start backup session")?;
        
        let completed_session = backup_manager.backup(session).await
            .context("Backup operation failed")?;

        // Report results
        let successful = completed_session.processed_files;
        let failed = completed_session.errors.len();

        println!("[✓] Backup completed: {} files successful, {} failed", successful, failed);
        
        if let Some(name) = &backup_name {
            println!("[✓] Backup name: {}", name);
        }
        if let Some(description) = &backup_description {
            println!("[✓] Description: {}", description);
        }
        
        println!("[✓] Backup directory: {}", completed_session.backup_dir.display());

        if failed > 0 {
            println!("Failed operations:");
            for error in &completed_session.errors {
                println!("  [✗] {}", error);
            }
        }

        Ok(())
    }

    /// Handle restore command
    async fn handle_restore(&mut self, args: &RestoreArgs) -> anyhow::Result<()> {
        info!("Starting restore operation");

        // Apply profile if specified
        if let Some(profile_name) = &args.profile {
            self.apply_profile(profile_name).await?;
        }

        // Determine backup directory from backup name or path
        let backup_dir = if args.backup.contains('/') || args.backup.contains('\\') {
            // Treat as path
            PathBuf::from(&args.backup)
        } else {
            // Treat as backup name - search for it
            self.find_backup_by_name(&args.backup).await?
        };

        if !backup_dir.exists() {
            return Err(anyhow::anyhow!("Backup directory not found: {}", backup_dir.display()));
        }

        // Load backup session metadata if available
        let session_metadata = self.load_backup_metadata(&backup_dir).await.ok();
        
        // Create filesystem and progress reporter
        let filesystem = if self.dry_run {
            FileSystemImpl::new_dry_run()
        } else {
            FileSystemImpl::new()
        };
        let progress_reporter = CliProgressReporter::new(self.verbose);

        // Create restore manager
        let restore_manager = RestoreManager::new(filesystem, progress_reporter, self.config.clone());

        // Perform restore based on whether this is a package restore or general restore
        let results = if let Some(package_name) = &args.package {
            // Package-based restore - restore specific files from the package
            if let Some(package_config) = self.config.get_package(package_name) {
                let mut all_results = Vec::new();
                
                // For each path in the package configuration
                for original_path in &package_config.paths {
                    if original_path.is_file() {
                        // Handle individual file
                        if let Some(backup_file_path) = self.find_file_in_backup(original_path, &backup_dir).await? {
                            let target_path = if args.in_place {
                                original_path.clone()
                            } else if let Some(ref target_dir) = args.target {
                                if original_path.is_absolute() {
                                    let relative_path = original_path.strip_prefix("/").unwrap_or(original_path);
                                    target_dir.join(relative_path)
                                } else {
                                    target_dir.join(original_path)
                                }
                            } else {
                                let relative_path = original_path.strip_prefix("/").unwrap_or(original_path);
                                PathBuf::from("./restored").join(relative_path)
                            };
                            
                            match self.restore_single_file(&backup_file_path, &target_path, args.overwrite).await {
                                Ok(()) => {
                                    all_results.push(OperationResult {
                                        operation_type: OperationType::Restore,
                                        path: target_path.clone(),
                                        success: true,
                                        error: None,
                                        details: Some(format!("Restored from {}", backup_file_path.display())),
                                        required_privileges: false,
                                        duration: None,
                                        bytes_processed: None,
                                    });
                                    println!("  [✓] Restored: {} -> {}", original_path.display(), target_path.display());
                                }
                                Err(e) => {
                                    all_results.push(OperationResult {
                                        operation_type: OperationType::Restore,
                                        path: target_path.clone(),
                                        success: false,
                                        error: Some(e.to_string()),
                                        details: None,
                                        required_privileges: false,
                                        duration: None,
                                        bytes_processed: None,
                                    });
                                    println!("  [✗] Failed to restore: {} -> {}: {}", original_path.display(), target_path.display(), e);
                                }
                            }
                        } else {
                            all_results.push(OperationResult {
                                operation_type: OperationType::Restore,
                                path: original_path.clone(),
                                success: false,
                                error: Some("File not found in backup".to_string()),
                                details: None,
                                required_privileges: false,
                                duration: None,
                                bytes_processed: None,
                            });
                            println!("  [✗] File not found in backup: {}", original_path.display());
                        }
                    } else if original_path.is_dir() {
                        // Handle directory - find all files that were backed up from this directory
                        let backed_up_files = self.find_all_files_in_backup_directory(original_path, &backup_dir).await?;
                        
                        if backed_up_files.is_empty() {
                            all_results.push(OperationResult {
                                operation_type: OperationType::Restore,
                                path: original_path.clone(),
                                success: false,
                                error: Some("No files found in backup for this directory".to_string()),
                                details: None,
                                required_privileges: false,
                                duration: None,
                                bytes_processed: None,
                            });
                            println!("  [✗] No files found in backup for directory: {}", original_path.display());
                        } else {
                            // Restore each file from the directory
                            for (backup_file_path, original_file_path) in backed_up_files {
                                let target_path = if args.in_place {
                                    original_file_path.clone()
                                } else if let Some(ref target_dir) = args.target {
                                    if original_file_path.is_absolute() {
                                        let relative_path = original_file_path.strip_prefix("/").unwrap_or(&original_file_path);
                                        target_dir.join(relative_path)
                                    } else {
                                        target_dir.join(&original_file_path)
                                    }
                                } else {
                                    let relative_path = original_file_path.strip_prefix("/").unwrap_or(&original_file_path);
                                    PathBuf::from("./restored").join(relative_path)
                                };
                                
                                match self.restore_single_file(&backup_file_path, &target_path, args.overwrite).await {
                                    Ok(()) => {
                                        all_results.push(OperationResult {
                                            operation_type: OperationType::Restore,
                                            path: target_path.clone(),
                                            success: true,
                                            error: None,
                                            details: Some(format!("Restored from {}", backup_file_path.display())),
                                            required_privileges: false,
                                            duration: None,
                                            bytes_processed: None,
                                        });
                                        println!("  [✓] Restored: {} -> {}", original_file_path.display(), target_path.display());
                                    }
                                    Err(e) => {
                                        all_results.push(OperationResult {
                                            operation_type: OperationType::Restore,
                                            path: target_path.clone(),
                                            success: false,
                                            error: Some(e.to_string()),
                                            details: None,
                                            required_privileges: false,
                                            duration: None,
                                            bytes_processed: None,
                                        });
                                        println!("  [✗] Failed to restore: {} -> {}: {}", original_file_path.display(), target_path.display(), e);
                                    }
                                }
                            }
                        }
                    } else {
                        // Path doesn't exist in current filesystem
                        all_results.push(OperationResult {
                            operation_type: OperationType::Restore,
                            path: original_path.clone(),
                            success: false,
                            error: Some("Path does not exist in current filesystem".to_string()),
                            details: None,
                            required_privileges: false,
                            duration: None,
                            bytes_processed: None,
                        });
                        println!("  [✗] Path does not exist: {}", original_path.display());
                    }
                }
                
                all_results
            } else {
                return Err(anyhow::anyhow!("Package '{}' not found in current configuration", package_name));
            }
        } else {
            // General restore - use the original restore manager logic
            let target_dir = args.target.clone().unwrap_or_else(|| {
                if args.in_place {
                    PathBuf::from("/") // Root for in-place restore
                } else {
                    PathBuf::from("./restored") // Default restore location
                }
            });
            
            restore_manager.restore_files(
                backup_dir.clone(),
                vec![target_dir],
            ).await.context("Restore operation failed")?
        };

        // Report results
        let successful = results.iter().filter(|r| r.success).count();
        let failed = results.iter().filter(|r| !r.success).count();

        println!("[✓] Restore completed: {} files successful, {} failed", successful, failed);
        
        if let Some(ref metadata) = session_metadata {
            if let Some(ref pkg_name) = metadata.package_name {
                println!("[✓] Package: {}", pkg_name);
            }
            if let Some(ref backup_name) = metadata.backup_name {
                println!("[✓] Backup name: {}", backup_name);
            }
        }

        if failed > 0 {
            println!("Failed operations:");
            for result in results.iter().filter(|r| !r.success) {
                println!("  [✗] {}: {}", 
                    result.path.display(), 
                    result.error.as_ref().unwrap_or(&"Unknown error".to_string())
                );
            }
        }

        Ok(())
    }

    /// Find backup directory by name (searches for matching backup names)
    async fn find_backup_by_name(&self, backup_name: &str) -> anyhow::Result<PathBuf> {
        let backup_root = &self.config.backup_dir;
        
        // First try exact match
        let exact_match = backup_root.join(backup_name);
        if exact_match.exists() {
            return Ok(exact_match);
        }
        
        // Then search for backups containing the name
        let mut entries = tokio::fs::read_dir(backup_root).await
            .context("Failed to read backup directory")?;
        
        let mut matches = Vec::new();
        while let Some(entry) = entries.next_entry().await? {
            let entry_name = entry.file_name().to_string_lossy().to_string();
            if entry_name.contains(backup_name) {
                matches.push((entry.path(), entry_name));
            }
        }
        
        match matches.len() {
            0 => Err(anyhow::anyhow!("No backup found matching name: {}", backup_name)),
            1 => Ok(matches[0].0.clone()),
            _ => {
                println!("Multiple backups found matching '{}':", backup_name);
                for (i, (_, name)) in matches.iter().enumerate() {
                    println!("  {}: {}", i + 1, name);
                }
                Err(anyhow::anyhow!("Multiple matches found. Please specify a more specific backup name."))
            }
        }
    }

    /// Load backup session metadata from backup directory
    async fn load_backup_metadata(&self, backup_dir: &Path) -> anyhow::Result<BackupSession> {
        let metadata_file = backup_dir.join("session_metadata.json");
        if !metadata_file.exists() {
            return Err(anyhow::anyhow!("No session metadata found in backup"));
        }
        
        let metadata_content = tokio::fs::read_to_string(&metadata_file).await
            .context("Failed to read session metadata")?;
        
        let session: BackupSession = serde_json::from_str(&metadata_content)
            .context("Failed to parse session metadata")?;
        
        Ok(session)
    }

    /// Handle list command
    async fn handle_list(&self, args: &ListArgs) -> anyhow::Result<()> {
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
            ListTarget::Packages => {
                let packages = &self.config.packages;
                
                if packages.is_empty() {
                    println!("No package configurations found");
                    println!("\nTo add a package configuration:");
                    println!("  dotman-rs package add <name> <paths...> --description \"Description\"");
                    println!("\nExample:");
                    println!("  dotman-rs package add nvim ~/.config/nvim --description \"Neovim configuration\"");
                    return Ok(());
                }

                println!("Available package configurations:");
                for (name, package_config) in packages {
                    println!("📦 {} - {}", name, package_config.description);
                    println!("   Paths ({}):", package_config.paths.len());
                    for path in &package_config.paths {
                        let exists_marker = if path.exists() { "✓" } else { "✗" };
                        println!("     {} {}", exists_marker, path.display());
                    }
                    if !package_config.exclude_patterns.is_empty() {
                        println!("   Exclude patterns: {:?}", package_config.exclude_patterns);
                    }
                    if !package_config.include_patterns.is_empty() {
                        println!("   Include patterns: {:?}", package_config.include_patterns);
                    }
                    println!();
                }
                
                println!("Usage examples:");
                for name in packages.keys() {
                    println!("  dotman-rs backup --package {}", name);
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
    async fn handle_verify(&self, args: &VerifyArgs) -> anyhow::Result<()> {
        let backup_path = self.resolve_backup_path(&args.backup).await?;
        
        let filesystem = FileSystemImpl::new();
        let progress_reporter = CliProgressReporter::new(self.verbose);
        let backup_engine = DefaultBackupEngine::new(filesystem, progress_reporter, self.config.clone());
        
        let is_valid = backup_engine.verify_backup(&backup_path).await
            .context("Verification failed")?;

        if is_valid {
            println!("[✓] Backup is valid");
        } else {
            println!("[✗] Backup verification failed");
            return Err(anyhow::anyhow!("Backup verification failed"));
        }

        Ok(())
    }

    /// Handle clean command
    async fn handle_clean(&self, args: &CleanArgs) -> anyhow::Result<()> {
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
            println!("[✓] Dry run: Would clean {} backups, freeing {} MB", cleaned_count, space_mb);
        } else {
            println!("[✓] Cleaned {} backups, freed {} MB", cleaned_count, space_mb);
        }

        Ok(())
    }

    /// Handle config command
    async fn handle_config(&mut self, args: &ConfigArgs) -> anyhow::Result<()> {
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
                        println!("[✓] Set backup_dir = {}", value);
                    },
                    "config_dir" => {
                        self.config.config_dir = PathBuf::from(value);
                        println!("[✓] Set config_dir = {}", value);
                    },
                    "max_backup_versions" => {
                        let versions: u32 = value.parse()
                            .context("Invalid number for max_backup_versions")?;
                        self.config.max_backup_versions = versions;
                        println!("[✓] Set max_backup_versions = {}", versions);
                    },
                    "log_level" => {
                        self.config.log_level = value.clone();
                        println!("[✓] Set log_level = {}", value);
                    },
                    "verify_integrity" => {
                        let verify: bool = value.parse()
                            .context("Invalid boolean for verify_integrity (use true/false)")?;
                        self.config.verify_integrity = verify;
                        println!("[✓] Set verify_integrity = {}", verify);
                    },
                    "preserve_permissions" => {
                        let preserve: bool = value.parse()
                            .context("Invalid boolean for preserve_permissions (use true/false)")?;
                        self.config.preserve_permissions = preserve;
                        println!("[✓] Set preserve_permissions = {}", preserve);
                    },
                    "create_backups" => {
                        let create: bool = value.parse()
                            .context("Invalid boolean for create_backups (use true/false)")?;
                        self.config.create_backups = create;
                        println!("[✓] Set create_backups = {}", create);
                    },
                    "follow_symlinks" => {
                        let follow: bool = value.parse()
                            .context("Invalid boolean for follow_symlinks (use true/false)")?;
                        self.config.follow_symlinks = follow;
                        println!("[✓] Set follow_symlinks = {}", follow);
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
                        println!("[✓] Cleared include_patterns");
                    },
                    "exclude_patterns" => {
                        self.config.exclude_patterns.clear();
                        println!("[✓] Cleared exclude_patterns");
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

                println!("[✓] Configuration reloaded");
            }
            ConfigAction::Validate => {
                match self.config.validate() {
                    Ok(_) => println!("[✓] Configuration is valid"),
                    Err(e) => {
                        println!("[✗] Configuration validation failed: {}", e);
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
                    
                    println!("[✓] Configuration reset to defaults");
                } else {
                    println!("Configuration reset cancelled");
                }
            }
        }

        Ok(())
    }

    /// Handle profile command
    async fn handle_profile(&mut self, args: &ProfileArgs) -> anyhow::Result<()> {
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
                
                println!("[✓] Created profile: {}", name);
            }
            ProfileAction::Delete { name, force } => {
                if *force || self.force || self.confirm(&format!("Delete profile '{}'?", name)).await? {
                    self.profile_manager.remove_profile(name).await
                        .context("Failed to delete profile")?;
                    println!("[✓] Deleted profile: {}", name);
                } else {
                    println!("Profile deletion cancelled");
                }
            }
            ProfileAction::Switch { name } => {
                self.profile_manager.set_active_profile(name).await
                    .context("Failed to switch profile")?;
                println!("[✓] Switched to profile: {}", name);
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
                println!("[✓] Renamed profile: {} -> {}", old_name, new_name);
            }
        }

        Ok(())
    }

    /// Handle package command
    async fn handle_package(&mut self, args: &PackageArgs) -> anyhow::Result<()> {
        match &args.action {
            PackageAction::List => {
                let packages = &self.config.packages;
                
                if packages.is_empty() {
                    println!("No package configurations found");
                    println!("\nTo add a package configuration:");
                    println!("  dotman-rs package add <name> <paths...> --description \"Description\"");
                    println!("\nExample:");
                    println!("  dotman-rs package add nvim ~/.config/nvim --description \"Neovim configuration\"");
                    return Ok(());
                }

                println!("Available package configurations:");
                for (name, package_config) in packages {
                    println!("📦 {} - {}", name, package_config.description);
                    println!("   Paths ({}):", package_config.paths.len());
                    for path in &package_config.paths {
                        let exists_marker = if path.exists() { "✓" } else { "✗" };
                        println!("     {} {}", exists_marker, path.display());
                    }
                    if !package_config.exclude_patterns.is_empty() {
                        println!("   Exclude patterns: {:?}", package_config.exclude_patterns);
                    }
                    if !package_config.include_patterns.is_empty() {
                        println!("   Include patterns: {:?}", package_config.include_patterns);
                    }
                    println!();
                }
                
                println!("Usage examples:");
                for name in packages.keys() {
                    println!("  dotman-rs backup --package {}", name);
                }
            }
            PackageAction::Add { name, description, paths, exclude, include } => {
                if self.config.packages.contains_key(name) {
                    if !self.force {
                        return Err(anyhow::anyhow!("Package '{}' already exists. Use --force to overwrite or 'package edit' to modify", name));
                    }
                }

                let package_description = description.clone().unwrap_or_else(|| format!("Package configuration for {}", name));
                
                let mut package_config = PackageConfig::new(
                    name.clone(),
                    package_description,
                    paths.clone(),
                );
                
                package_config.exclude_patterns = exclude.clone();
                package_config.include_patterns = include.clone();
                
                self.config.set_package(package_config);
                
                // Save configuration
                let config_path = self.config.config_dir.join("config.toml");
                self.config.save(&config_path).await
                    .context("Failed to save configuration")?;
                
                println!("[✓] Added package configuration: {}", name);
                println!("    Description: {}", self.config.get_package(name).unwrap().description);
                println!("    Paths: {}", paths.len());
                for path in paths {
                    println!("      {}", path.display());
                }
                if !exclude.is_empty() {
                    println!("    Exclude patterns: {:?}", exclude);
                }
                if !include.is_empty() {
                    println!("    Include patterns: {:?}", include);
                }
            }
            PackageAction::Remove { name, force } => {
                if !self.config.packages.contains_key(name) {
                    return Err(anyhow::anyhow!("Package '{}' does not exist", name));
                }

                if !force && !self.force {
                    let confirm = self.confirm(&format!("Remove package configuration '{}'?", name)).await?;
                    if !confirm {
                        println!("Operation cancelled");
                        return Ok(());
                    }
                }

                self.config.remove_package(name);
                
                // Save configuration
                let config_path = self.config.config_dir.join("config.toml");
                self.config.save(&config_path).await
                    .context("Failed to save configuration")?;
                
                println!("[✓] Removed package configuration: {}", name);
            }
            PackageAction::Show { name } => {
                if let Some(package_config) = self.config.get_package(name) {
                    println!("Package: {}", name);
                    println!("Description: {}", package_config.description);
                    println!("Paths ({}):", package_config.paths.len());
                    for path in &package_config.paths {
                        let exists_marker = if path.exists() { "✓" } else { "✗" };
                        println!("  {} {}", exists_marker, path.display());
                    }
                    if !package_config.exclude_patterns.is_empty() {
                        println!("Exclude patterns:");
                        for pattern in &package_config.exclude_patterns {
                            println!("  {}", pattern);
                        }
                    }
                    if !package_config.include_patterns.is_empty() {
                        println!("Include patterns:");
                        for pattern in &package_config.include_patterns {
                            println!("  {}", pattern);
                        }
                    }
                    
                    println!("\nUsage:");
                    println!("  dotman-rs backup --package {}", name);
                } else {
                    return Err(anyhow::anyhow!("Package '{}' does not exist", name));
                }
            }
            PackageAction::Edit { 
                name, 
                description, 
                add_paths, 
                remove_paths, 
                add_exclude, 
                remove_exclude, 
                add_include, 
                remove_include 
            } => {
                if !self.config.packages.contains_key(name) {
                    return Err(anyhow::anyhow!("Package '{}' does not exist", name));
                }

                let mut package_config = self.config.get_package(name).unwrap().clone();
                let mut changes_made = false;

                // Update description
                if let Some(new_desc) = description {
                    package_config.description = new_desc.clone();
                    changes_made = true;
                    println!("Updated description: {}", new_desc);
                }

                // Add paths
                for path in add_paths {
                    if !package_config.paths.contains(path) {
                        package_config.paths.push(path.clone());
                        changes_made = true;
                        println!("Added path: {}", path.display());
                    } else {
                        println!("Path already exists: {}", path.display());
                    }
                }

                // Remove paths
                for path in remove_paths {
                    if let Some(pos) = package_config.paths.iter().position(|p| p == path) {
                        package_config.paths.remove(pos);
                        changes_made = true;
                        println!("Removed path: {}", path.display());
                    } else {
                        println!("Path not found: {}", path.display());
                    }
                }

                // Add exclude patterns
                for pattern in add_exclude {
                    if !package_config.exclude_patterns.contains(pattern) {
                        package_config.exclude_patterns.push(pattern.clone());
                        changes_made = true;
                        println!("Added exclude pattern: {}", pattern);
                    } else {
                        println!("Exclude pattern already exists: {}", pattern);
                    }
                }

                // Remove exclude patterns
                for pattern in remove_exclude {
                    if let Some(pos) = package_config.exclude_patterns.iter().position(|p| p == pattern) {
                        package_config.exclude_patterns.remove(pos);
                        changes_made = true;
                        println!("Removed exclude pattern: {}", pattern);
                    } else {
                        println!("Exclude pattern not found: {}", pattern);
                    }
                }

                // Add include patterns
                for pattern in add_include {
                    if !package_config.include_patterns.contains(pattern) {
                        package_config.include_patterns.push(pattern.clone());
                        changes_made = true;
                        println!("Added include pattern: {}", pattern);
                    } else {
                        println!("Include pattern already exists: {}", pattern);
                    }
                }

                // Remove include patterns
                for pattern in remove_include {
                    if let Some(pos) = package_config.include_patterns.iter().position(|p| p == pattern) {
                        package_config.include_patterns.remove(pos);
                        changes_made = true;
                        println!("Removed include pattern: {}", pattern);
                    } else {
                        println!("Include pattern not found: {}", pattern);
                    }
                }

                if changes_made {
                    self.config.set_package(package_config);
                    
                    // Save configuration
                    let config_path = self.config.config_dir.join("config.toml");
                    self.config.save(&config_path).await
                        .context("Failed to save configuration")?;
                    
                    println!("[✓] Updated package configuration: {}", name);
                } else {
                    println!("No changes made to package: {}", name);
                }
            }
        }

        Ok(())
    }

    /// Handle status command - show changes since last backup
    async fn handle_status(&mut self, args: &StatusArgs) -> anyhow::Result<()> {
        info!("Checking status");

        // Apply profile if specified
        if let Some(profile_name) = &args.profile {
            self.apply_profile(profile_name).await?;
        }

        // Determine what to check status for
        let (check_paths, package_name) = if let Some(package_name) = &args.package {
            // Package-based status
            if let Some(package_config) = self.config.get_package(package_name) {
                println!("[📦] Checking status for package: {}", package_name);
                println!("[📦] Package description: {}", package_config.description);
                (package_config.paths.clone(), Some(package_name.clone()))
            } else {
                return Err(anyhow::anyhow!("Package '{}' not found in configuration", package_name));
            }
        } else if !args.paths.is_empty() {
            // Path-based status
            (args.paths.clone(), None)
        } else {
            return Err(anyhow::anyhow!("Either --package or paths must be specified"));
        };

        // Find the most recent backup for this package/paths
        let backup_dir = if let Some(backup_name) = &args.backup {
            // Use specific backup
            self.find_backup_by_name(backup_name).await?
        } else {
            // Find most recent backup for this package
            self.find_most_recent_backup(&package_name).await?
        };

        // Load backup metadata
        let backup_metadata = self.load_backup_metadata(&backup_dir).await?;
        
        println!("\n[📊] Status Report");
        println!("Backup: {}", backup_dir.file_name().unwrap().to_string_lossy());
        println!("Created: {}", backup_metadata.started_at.format("%Y-%m-%d %H:%M:%S UTC"));
        if let Some(ref pkg_name) = backup_metadata.package_name {
            println!("Package: {}", pkg_name);
        }
        if let Some(ref description) = backup_metadata.backup_description {
            println!("Description: {}", description);
        }

        // Check each path for changes
        let mut total_files = 0;
        let mut changed_files = 0;
        let mut new_files = 0;
        let mut missing_files = 0;

        for path in &check_paths {
            if path.is_file() {
                total_files += 1;
                match self.check_file_status(path, &backup_dir).await? {
                    FileStatus::Unchanged => {
                        if args.detailed && !args.changed {
                            println!("  [=] {}", path.display());
                        }
                    }
                    FileStatus::Modified => {
                        changed_files += 1;
                        println!("  [M] {}", path.display());
                    }
                    FileStatus::New => {
                        new_files += 1;
                        println!("  [+] {}", path.display());
                    }
                    FileStatus::Missing => {
                        missing_files += 1;
                        println!("  [-] {}", path.display());
                    }
                }
            } else if path.is_dir() {
                let (dir_total, dir_changed, dir_new, dir_missing) = 
                    self.check_directory_status(path, &backup_dir, args.detailed, args.changed).await?;
                total_files += dir_total;
                changed_files += dir_changed;
                new_files += dir_new;
                missing_files += dir_missing;
            }
        }

        // Summary
        println!("\n[📊] Summary:");
        println!("  Total files: {}", total_files);
        println!("  Changed: {}", changed_files);
        println!("  New: {}", new_files);
        println!("  Missing: {}", missing_files);
        println!("  Unchanged: {}", total_files - changed_files - new_files - missing_files);

        if changed_files > 0 || new_files > 0 {
            println!("\n[💡] Tip: Run 'dotman-rs backup --package {}' to create a new backup with current changes", 
                package_name.as_deref().unwrap_or("your-files"));
        }

        Ok(())
    }

    /// Handle diff command - show detailed differences
    async fn handle_diff(&mut self, args: &DiffArgs) -> anyhow::Result<()> {
        info!("Showing differences");

        // Find backup directory
        let backup_dir = self.find_backup_by_name(&args.backup).await?;
        let backup_metadata = self.load_backup_metadata(&backup_dir).await?;

        println!("[🔍] Comparing current state with backup: {}", args.backup);
        println!("Backup created: {}", backup_metadata.started_at.format("%Y-%m-%d %H:%M:%S UTC"));
        
        if let Some(ref pkg_name) = backup_metadata.package_name {
            println!("Package: {}", pkg_name);
        }

        // Determine what to compare
        let compare_paths = if let Some(package_name) = &args.package {
            if let Some(package_config) = self.config.get_package(package_name) {
                package_config.paths.clone()
            } else {
                return Err(anyhow::anyhow!("Package '{}' not found", package_name));
            }
        } else if !args.files.is_empty() {
            args.files.clone()
        } else {
            // Use paths from backup metadata
            backup_metadata.source_paths.clone()
        };

        println!("\n[🔍] Differences:");

        for path in &compare_paths {
            if path.is_file() {
                self.show_file_diff(path, &backup_dir, args.show_timestamps, args.show_identical).await?;
            } else if path.is_dir() {
                self.show_directory_diff(path, &backup_dir, args.show_timestamps, args.show_identical).await?;
            }
        }

        Ok(())
    }

    /// Find the most recent backup for a package or general backup
    async fn find_most_recent_backup(&self, package_name: &Option<String>) -> anyhow::Result<PathBuf> {
        let backup_root = &self.config.backup_dir;
        let mut entries = tokio::fs::read_dir(backup_root).await
            .context("Failed to read backup directory")?;
        
        let mut backups = Vec::new();
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.is_dir() {
                // Try to load metadata to check if it matches our criteria
                if let Ok(metadata) = self.load_backup_metadata(&path).await {
                    if let Some(ref pkg_name) = package_name {
                        if metadata.package_name.as_ref() == Some(pkg_name) {
                            backups.push((path, metadata.started_at));
                        }
                    } else {
                        backups.push((path, metadata.started_at));
                    }
                }
            }
        }

        if backups.is_empty() {
            return Err(anyhow::anyhow!("No backups found for the specified criteria"));
        }

        // Sort by date (most recent first)
        backups.sort_by(|a, b| b.1.cmp(&a.1));
        Ok(backups[0].0.clone())
    }

    /// Check the status of a single file
    async fn check_file_status(&self, file_path: &Path, backup_dir: &Path) -> anyhow::Result<FileStatus> {
        // Find the corresponding file in backup
        let backup_file = self.find_file_in_backup(file_path, backup_dir).await?;
        
        if !file_path.exists() {
            return Ok(FileStatus::Missing);
        }
        
        if backup_file.is_none() {
            return Ok(FileStatus::New);
        }
        
        let backup_file = backup_file.unwrap();
        
        // Compare file metadata first for quick checks
        let current_metadata = tokio::fs::metadata(file_path).await?;
        let backup_metadata = tokio::fs::metadata(&backup_file).await?;
        
        // Quick check: if sizes differ, file is definitely modified
        if current_metadata.len() != backup_metadata.len() {
            return Ok(FileStatus::Modified);
        }
        
        // If sizes are the same, compare actual file content using SHA-256 hashes
        // This is the proper way to detect if files are truly identical
        let current_hash = self.calculate_file_hash(file_path).await?;
        let backup_hash = self.calculate_file_hash(&backup_file).await?;
        
        if current_hash == backup_hash {
            Ok(FileStatus::Unchanged)
        } else {
            Ok(FileStatus::Modified)
        }
    }

    /// Calculate SHA-256 hash of a file for content comparison
    async fn calculate_file_hash(&self, file_path: &Path) -> anyhow::Result<String> {
        use sha2::{Sha256, Digest};
        
        let content = tokio::fs::read(file_path).await
            .context(format!("Failed to read file for hashing: {}", file_path.display()))?;
        
        let mut hasher = Sha256::new();
        hasher.update(&content);
        let hash = hasher.finalize();
        
        Ok(format!("{:x}", hash))
    }

    /// Check status of a directory recursively
    fn check_directory_status<'a>(
        &'a self, 
        dir_path: &'a Path, 
        backup_dir: &'a Path, 
        detailed: bool, 
        changed_only: bool
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = anyhow::Result<(u64, u64, u64, u64)>> + Send + 'a>> {
        Box::pin(async move {
            let mut total = 0;
            let mut changed = 0;
            let mut new = 0;
            let mut missing = 0;

            if !dir_path.exists() {
                return Ok((0, 0, 0, 0));
            }

            let mut entries = tokio::fs::read_dir(dir_path).await?;
            while let Some(entry) = entries.next_entry().await? {
                let path = entry.path();
                if path.is_file() {
                    total += 1;
                    match self.check_file_status(&path, backup_dir).await? {
                        FileStatus::Unchanged => {
                            if detailed && !changed_only {
                                println!("  [=] {}", path.display());
                            }
                        }
                        FileStatus::Modified => {
                            changed += 1;
                            println!("  [M] {}", path.display());
                        }
                        FileStatus::New => {
                            new += 1;
                            println!("  [+] {}", path.display());
                        }
                        FileStatus::Missing => {
                            missing += 1;
                            println!("  [-] {}", path.display());
                        }
                    }
                } else if path.is_dir() {
                    let (sub_total, sub_changed, sub_new, sub_missing) = 
                        self.check_directory_status(&path, backup_dir, detailed, changed_only).await?;
                    total += sub_total;
                    changed += sub_changed;
                    new += sub_new;
                    missing += sub_missing;
                }
            }

            Ok((total, changed, new, missing))
        })
    }

    /// Find a file in the backup directory structure
    async fn find_file_in_backup(&self, file_path: &Path, backup_dir: &Path) -> anyhow::Result<Option<PathBuf>> {
        // The backup stores files with their full path structure
        // For example, /home/user/.bashrc becomes backup_dir/home/user/.bashrc
        
        // Convert the file path to be relative to the backup directory
        let backup_file_path = if file_path.is_absolute() {
            // Remove the leading slash and join with backup_dir
            let path_without_root = file_path.strip_prefix("/").unwrap_or(file_path);
            backup_dir.join(path_without_root)
        } else {
            // For relative paths, just join directly
            backup_dir.join(file_path)
        };
        
        if backup_file_path.exists() {
            Ok(Some(backup_file_path))
        } else {
            Ok(None)
        }
    }

    /// Find all files that were backed up from a specific directory
    async fn find_all_files_in_backup_directory(&self, dir_path: &Path, backup_dir: &Path) -> anyhow::Result<Vec<(PathBuf, PathBuf)>> {
        let mut backed_up_files = Vec::new();
        
        // Convert the directory path to backup path
        let backup_dir_path = if dir_path.is_absolute() {
            let path_without_root = dir_path.strip_prefix("/").unwrap_or(dir_path);
            backup_dir.join(path_without_root)
        } else {
            backup_dir.join(dir_path)
        };
        
        if !backup_dir_path.exists() {
            return Ok(backed_up_files);
        }
        
        // Recursively find all files in the backup directory
        self.collect_backup_files_recursive(&backup_dir_path, dir_path, &mut backed_up_files).await?;
        
        Ok(backed_up_files)
    }

    /// Recursively collect all files from a backup directory
    fn collect_backup_files_recursive<'a>(
        &'a self, 
        backup_dir_path: &'a Path, 
        original_base_path: &'a Path, 
        files: &'a mut Vec<(PathBuf, PathBuf)>
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = anyhow::Result<()>> + Send + 'a>> {
        Box::pin(async move {
            let mut entries = tokio::fs::read_dir(backup_dir_path).await?;
            
            while let Some(entry) = entries.next_entry().await? {
                let backup_file_path = entry.path();
                
                if backup_file_path.is_file() {
                    // Calculate the original file path by replacing the backup base with the original base
                    let relative_path = backup_file_path.strip_prefix(backup_dir_path)
                        .unwrap_or(&backup_file_path);
                    let original_file_path = original_base_path.join(relative_path);
                    
                    files.push((backup_file_path, original_file_path));
                } else if backup_file_path.is_dir() {
                    // Recursively process subdirectories
                    let relative_path = backup_file_path.strip_prefix(backup_dir_path)
                        .unwrap_or(&backup_file_path);
                    let original_subdir_path = original_base_path.join(relative_path);
                    
                    self.collect_backup_files_recursive(&backup_file_path, &original_subdir_path, files).await?;
                }
            }
            
            Ok(())
        })
    }

    /// Show differences for a single file
    async fn show_file_diff(&self, file_path: &Path, backup_dir: &Path, show_timestamps: bool, show_identical: bool) -> anyhow::Result<()> {
        let status = self.check_file_status(file_path, backup_dir).await?;
        
        match status {
            FileStatus::Unchanged => {
                if show_identical {
                    println!("  [=] {} (identical)", file_path.display());
                }
            }
            FileStatus::Modified => {
                println!("  [M] {} (modified)", file_path.display());
                if show_timestamps {
                    if let Ok(metadata) = tokio::fs::metadata(file_path).await {
                        if let Ok(modified) = metadata.modified() {
                            println!("      Modified: {:?}", modified);
                        }
                    }
                }
            }
            FileStatus::New => {
                println!("  [+] {} (new file)", file_path.display());
            }
            FileStatus::Missing => {
                println!("  [-] {} (missing from current)", file_path.display());
            }
        }
        
        Ok(())
    }

    /// Show diff for a directory recursively
    fn show_directory_diff<'a>(
        &'a self, 
        dir_path: &'a Path, 
        backup_dir: &'a Path, 
        show_timestamps: bool, 
        show_identical: bool
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = anyhow::Result<()>> + Send + 'a>> {
        Box::pin(async move {
            if !dir_path.exists() {
                return Ok(());
            }

            let mut entries = tokio::fs::read_dir(dir_path).await?;
            while let Some(entry) = entries.next_entry().await? {
                let path = entry.path();
                if path.is_file() {
                    match self.check_file_status(&path, backup_dir).await? {
                        FileStatus::Unchanged => {
                            if show_identical {
                                if show_timestamps {
                                    let metadata = tokio::fs::metadata(&path).await?;
                                    let modified = metadata.modified()?;
                                    let modified_dt: chrono::DateTime<chrono::Local> = modified.into();
                                    println!("  [=] {} ({})", path.display(), modified_dt.format("%Y-%m-%d %H:%M:%S"));
                                } else {
                                    println!("  [=] {}", path.display());
                                }
                            }
                        }
                        FileStatus::Modified => {
                            if show_timestamps {
                                let metadata = tokio::fs::metadata(&path).await?;
                                let modified = metadata.modified()?;
                                let modified_dt: chrono::DateTime<chrono::Local> = modified.into();
                                println!("  [M] {} ({})", path.display(), modified_dt.format("%Y-%m-%d %H:%M:%S"));
                            } else {
                                println!("  [M] {}", path.display());
                            }
                        }
                        FileStatus::New => {
                            if show_timestamps {
                                let metadata = tokio::fs::metadata(&path).await?;
                                let modified = metadata.modified()?;
                                let modified_dt: chrono::DateTime<chrono::Local> = modified.into();
                                println!("  [+] {} ({})", path.display(), modified_dt.format("%Y-%m-%d %H:%M:%S"));
                            } else {
                                println!("  [+] {}", path.display());
                            }
                        }
                        FileStatus::Missing => {
                            println!("  [-] {} (missing from current)", path.display());
                        }
                    }
                } else if path.is_dir() {
                    self.show_directory_diff(&path, backup_dir, show_timestamps, show_identical).await?;
                }
            }

            Ok(())
        })
    }

    /// Apply a profile to current configuration
    async fn apply_profile(&mut self, profile_name: &str) -> anyhow::Result<()> {
        let profile = self.profile_manager.get_profile(profile_name)
            .ok_or_else(|| anyhow::anyhow!("Profile '{}' not found", profile_name))?;

        // Apply profile overrides to config
        self.config = profile.get_effective_config(None);
        
        Ok(())
    }

    /// Resolve backup path from name or path
    async fn resolve_backup_path(&self, backup: &str) -> anyhow::Result<PathBuf> {
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
    async fn confirm(&self, message: &str) -> anyhow::Result<bool> {
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
    fn calculate_directory_size<'a>(&'a self, dir_path: &'a std::path::Path) -> std::pin::Pin<Box<dyn std::future::Future<Output = anyhow::Result<u64>> + Send + 'a>> {
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
    fn count_files_in_directory<'a>(&'a self, dir_path: &'a std::path::Path) -> std::pin::Pin<Box<dyn std::future::Future<Output = anyhow::Result<u64>> + Send + 'a>> {
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

    /// Restore a single file from backup to target location
    async fn restore_single_file(&self, backup_file_path: &Path, target_path: &Path, overwrite: bool) -> anyhow::Result<()> {
        // Check if target file exists
        if target_path.exists() && !overwrite {
            return Err(anyhow::anyhow!("Target file exists and overwrite not specified: {}", target_path.display()));
        }
        
        // Create parent directory if needed
        if let Some(parent) = target_path.parent() {
            tokio::fs::create_dir_all(parent).await
                .context(format!("Failed to create parent directory: {}", parent.display()))?;
        }
        
        // Copy file from backup to target
        tokio::fs::copy(backup_file_path, target_path).await
            .context(format!("Failed to copy {} to {}", backup_file_path.display(), target_path.display()))?;
        
        Ok(())
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