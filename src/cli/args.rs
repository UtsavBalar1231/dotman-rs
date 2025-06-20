use clap::{Parser, Subcommand, Args};
use std::path::PathBuf;

/// Dotman - A comprehensive dotfile management tool
#[derive(Parser, Debug)]
#[command(name = "dotman")]
#[command(about = "A comprehensive dotfile management tool", long_about = None)]
#[command(version)]
pub struct DotmanArgs {
    /// Enable verbose logging
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// Configuration file path
    #[arg(short, long)]
    pub config: Option<PathBuf>,

    /// Dry run mode - show what would be done without executing
    #[arg(short = 'n', long)]
    pub dry_run: bool,

    /// Force operations without confirmation
    #[arg(short, long)]
    pub force: bool,

    /// Enable interactive mode
    #[arg(short, long)]
    pub interactive: bool,

    /// Working directory
    #[arg(short = 'C', long)]
    pub directory: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Initialize dotman configuration
    Init(InitArgs),
    
    /// Backup dotfiles
    Backup(BackupArgs),
    
    /// Restore dotfiles from backup
    Restore(RestoreArgs),
    
    /// List available backups or backup contents
    List(ListArgs),
    
    /// Verify backup integrity
    Verify(VerifyArgs),
    
    /// Clean up old backups
    Clean(CleanArgs),
    
    /// Configuration management
    Config(ConfigArgs),
    
    /// Profile management
    Profile(ProfileArgs),
    
    /// Show status of dotfiles
    Status(StatusArgs),
    
    /// Compare dotfiles with backup
    Diff(DiffArgs),
}

#[derive(Args, Debug)]
pub struct InitArgs {
    /// Target directory for dotman configuration
    #[arg(short, long)]
    pub target: Option<PathBuf>,
    
    /// Initialize with default configuration
    #[arg(short, long)]
    pub defaults: bool,
    
    /// Backup directory path
    #[arg(short, long)]
    pub backup_dir: Option<PathBuf>,
    
    /// Initialize for specific profile
    #[arg(short, long)]
    pub profile: Option<String>,
}

#[derive(Args, Debug)]
pub struct BackupArgs {
    /// Paths to backup
    pub paths: Vec<PathBuf>,
    
    /// Backup name/tag
    #[arg(short, long)]
    pub name: Option<String>,
    
    /// Backup description
    #[arg(short, long)]
    pub description: Option<String>,
    
    /// Include hidden files
    #[arg(long)]
    pub include_hidden: bool,
    
    /// Follow symlinks
    #[arg(long)]
    pub follow_links: bool,
    
    /// Exclude patterns (glob)
    #[arg(short, long, action = clap::ArgAction::Append)]
    pub exclude: Vec<String>,
    
    /// Include patterns (glob)
    #[arg(long, action = clap::ArgAction::Append)]
    pub include: Vec<String>,
    
    /// Verify backup after creation
    #[arg(short, long)]
    pub verify: bool,
    
    /// Compress backup
    #[arg(short, long)]
    pub compress: bool,
    
    /// Encrypt backup
    #[arg(long)]
    pub encrypt: bool,
    
    /// Use specific profile
    #[arg(short, long)]
    pub profile: Option<String>,
}

#[derive(Args, Debug)]
pub struct RestoreArgs {
    /// Backup path or name to restore from
    pub backup: String,
    
    /// Target directory for restoration
    #[arg(short, long)]
    pub target: Option<PathBuf>,
    
    /// Specific files/paths to restore
    pub files: Vec<PathBuf>,
    
    /// Overwrite existing files
    #[arg(long)]
    pub overwrite: bool,
    
    /// Create backup of existing files before restore
    #[arg(long)]
    pub backup_existing: bool,
    
    /// Restore to original locations
    #[arg(long)]
    pub in_place: bool,
    
    /// Preserve file permissions
    #[arg(long)]
    pub preserve_permissions: bool,
    
    /// Preserve ownership (requires privileges)
    #[arg(long)]
    pub preserve_ownership: bool,
    
    /// Use specific profile
    #[arg(short, long)]
    pub profile: Option<String>,
}

#[derive(Args, Debug)]
pub struct ListArgs {
    /// What to list
    #[command(subcommand)]
    pub target: ListTarget,
}

#[derive(Subcommand, Debug)]
pub enum ListTarget {
    /// List available backups
    Backups,
    
    /// List contents of a specific backup
    Contents {
        /// Backup path or name
        backup: String,
        
        /// Show detailed information
        #[arg(short, long)]
        detailed: bool,
        
        /// Filter by file type
        #[arg(short, long)]
        filter: Option<String>,
    },
    
    /// List available profiles
    Profiles,
    
    /// List configuration entries
    Config,
}

#[derive(Args, Debug)]
pub struct VerifyArgs {
    /// Backup path or name to verify
    pub backup: String,
    
    /// Verify file checksums
    #[arg(short, long)]
    pub checksums: bool,
    
    /// Verify file permissions
    #[arg(short, long)]
    pub permissions: bool,
    
    /// Detailed verification report
    #[arg(short, long)]
    pub detailed: bool,
}

#[derive(Args, Debug)]
pub struct CleanArgs {
    /// Number of backups to keep
    #[arg(short, long)]
    pub keep: Option<usize>,
    
    /// Remove backups older than specified days
    #[arg(long)]
    pub older_than: Option<u32>,
    
    /// Remove corrupted backups
    #[arg(long)]
    pub corrupted: bool,
    
    /// Remove temporary files
    #[arg(long)]
    pub temp: bool,
    
    /// Show what would be removed without actually removing
    #[arg(long)]
    pub dry_run: bool,
}

#[derive(Args, Debug)]
pub struct ConfigArgs {
    /// Configuration command
    #[command(subcommand)]
    pub action: ConfigAction,
}

#[derive(Subcommand, Debug)]
pub enum ConfigAction {
    /// Show current configuration
    Show {
        /// Show specific key
        key: Option<String>,
    },
    
    /// Set configuration value
    Set {
        /// Configuration key
        key: String,
        
        /// Configuration value
        value: String,
    },
    
    /// Get configuration value
    Get {
        /// Configuration key
        key: String,
    },
    
    /// Remove configuration key
    Unset {
        /// Configuration key
        key: String,
    },
    
    /// Edit configuration file
    Edit,
    
    /// Validate configuration
    Validate,
    
    /// Reset to defaults
    Reset {
        /// Confirm reset
        #[arg(short, long)]
        confirm: bool,
    },
}

#[derive(Args, Debug)]
pub struct ProfileArgs {
    /// Profile command
    #[command(subcommand)]
    pub action: ProfileAction,
}

#[derive(Subcommand, Debug)]
pub enum ProfileAction {
    /// List available profiles
    List,
    
    /// Create new profile
    Create {
        /// Profile name
        name: String,
        
        /// Profile description
        #[arg(short, long)]
        description: Option<String>,
        
        /// Copy from existing profile
        #[arg(short, long)]
        from: Option<String>,
    },
    
    /// Delete profile
    Delete {
        /// Profile name
        name: String,
        
        /// Force deletion without confirmation
        #[arg(short, long)]
        force: bool,
    },
    
    /// Switch to profile
    Switch {
        /// Profile name
        name: String,
    },
    
    /// Show current profile
    Current,
    
    /// Edit profile
    Edit {
        /// Profile name
        name: String,
    },
    
    /// Rename profile
    Rename {
        /// Current profile name
        old_name: String,
        
        /// New profile name
        new_name: String,
    },
}

#[derive(Args, Debug)]
pub struct StatusArgs {
    /// Paths to check status for
    pub paths: Vec<PathBuf>,
    
    /// Show detailed status
    #[arg(short, long)]
    pub detailed: bool,
    
    /// Check against specific backup
    #[arg(short, long)]
    pub backup: Option<String>,
    
    /// Show only changed files
    #[arg(short, long)]
    pub changed: bool,
    
    /// Use specific profile
    #[arg(short, long)]
    pub profile: Option<String>,
}

#[derive(Args, Debug)]
pub struct DiffArgs {
    /// First path/backup to compare
    pub source: String,
    
    /// Second path/backup to compare (optional, defaults to current state)
    pub target: Option<String>,
    
    /// Show context lines
    #[arg(short, long, default_value = "3")]
    pub context: usize,
    
    /// Use unified diff format
    #[arg(short, long)]
    pub unified: bool,
    
    /// Compare only specific files
    pub files: Vec<PathBuf>,
    
    /// Ignore whitespace differences
    #[arg(short, long)]
    pub ignore_whitespace: bool,
}

/// Parse command line arguments
pub fn parse_args() -> DotmanArgs {
    DotmanArgs::parse()
}

/// Parse arguments from custom iterator (useful for testing)
pub fn parse_args_from<I, T>(args: I) -> DotmanArgs 
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    DotmanArgs::parse_from(args)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_init_command() {
        let args = parse_args_from([
            "dotman",
            "init",
            "--target", "/home/user/.config/dotman",
            "--defaults"
        ]);

        match args.command {
            Command::Init(init_args) => {
                assert_eq!(init_args.target, Some(PathBuf::from("/home/user/.config/dotman")));
                assert!(init_args.defaults);
            }
            _ => panic!("Expected Init command"),
        }
    }

    #[test]
    fn test_parse_backup_command() {
        let args = parse_args_from([
            "dotman",
            "backup",
            "/home/user/.bashrc",
            "/home/user/.vimrc",
            "--name", "my-backup",
            "--exclude", "*.log",
            "--verify"
        ]);

        match args.command {
            Command::Backup(backup_args) => {
                assert_eq!(backup_args.paths.len(), 2);
                assert_eq!(backup_args.name, Some("my-backup".to_string()));
                assert_eq!(backup_args.exclude, vec!["*.log"]);
                assert!(backup_args.verify);
            }
            _ => panic!("Expected Backup command"),
        }
    }

    #[test]
    fn test_parse_restore_command() {
        let args = parse_args_from([
            "dotman",
            "restore",
            "my-backup",
            "--target", "/tmp/restore",
            "--overwrite"
        ]);

        match args.command {
            Command::Restore(restore_args) => {
                assert_eq!(restore_args.backup, "my-backup");
                assert_eq!(restore_args.target, Some(PathBuf::from("/tmp/restore")));
                assert!(restore_args.overwrite);
            }
            _ => panic!("Expected Restore command"),
        }
    }

    #[test]
    fn test_parse_config_command() {
        let args = parse_args_from([
            "dotman",
            "config",
            "set",
            "backup.compression",
            "true"
        ]);

        match args.command {
            Command::Config(config_args) => {
                match config_args.action {
                    ConfigAction::Set { key, value } => {
                        assert_eq!(key, "backup.compression");
                        assert_eq!(value, "true");
                    }
                    _ => panic!("Expected Set config action"),
                }
            }
            _ => panic!("Expected Config command"),
        }
    }

    #[test]
    fn test_global_flags() {
        let args = parse_args_from([
            "dotman",
            "-vv",
            "--dry-run",
            "--force",
            "init"
        ]);

        assert_eq!(args.verbose, 2);
        assert!(args.dry_run);
        assert!(args.force);
    }
} 