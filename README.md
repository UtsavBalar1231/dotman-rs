# dotman-rs

A comprehensive dotfiles management system written in Rust, providing advanced backup, restore, and synchronization capabilities for configuration files with enterprise-grade features including transaction safety, compression, encryption, and integrity verification.

## Table of Contents

- [Features](#features)
- [Installation](#installation)
- [Quick Start](#quick-start)
- [Comprehensive Usage Guide](#comprehensive-usage-guide)
- [Configuration](#configuration)
- [Advanced Usage](#advanced-usage)
- [Use Cases and Scenarios](#use-cases-and-scenarios)
- [Troubleshooting](#troubleshooting)
- [Development](#development)
- [Contributing](#contributing)
- [License](#license)

## Features

### Core Capabilities
- **Backup & Restore**: Comprehensive backup and restore operations with metadata preservation
- **Profile Management**: Organize configurations using multiple profiles for different environments
- **Security**: Optional encryption and integrity verification with BLAKE3 hashing
- **Compression**: Built-in compression support to minimize backup sizes
- **Performance**: Asynchronous operations with parallel processing for large file sets
- **Symlink Support**: Intelligent handling of symbolic links with loop detection
- **Transaction Safety**: Atomic operations with rollback capabilities
- **Progress Tracking**: Real-time progress reporting for long-running operations
- **Configuration Management**: Flexible configuration system with validation and migration
- **Pattern Matching**: Include/exclude patterns using glob syntax
- **Permission Handling**: Proper Unix permission and ownership preservation

### Enterprise Features
- **Dry-run Mode**: Test operations without making changes
- **Verbose Logging**: Multiple logging levels for debugging and monitoring
- **Cross-platform**: Linux, macOS, Windows support (Unix features work best on Unix-like systems)
- **Extensible Architecture**: Trait-based design for custom implementations
- **Comprehensive Testing**: 60+ unit tests ensuring reliability

## Installation

### Prerequisites

- Rust 1.70 or later
- Git (for source installation)

### From Source

1. **Install Rust** using [rustup](https://rustup.rs/):
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   source ~/.cargo/env
   ```

2. **Clone and build**:
   ```bash
   git clone https://github.com/UtsavBalar1231/dotman-rs.git
   cd dotman-rs
   cargo install --path .
   ```

3. **Verify installation**:
   ```bash
   dotman-rs --version
   ```

### From Releases (Coming Soon)

Pre-built binaries will be available on the [releases page](https://github.com/UtsavBalar1231/dotman-rs/releases).

## Quick Start

### For New Users

1. **Initialize dotman**:
   ```bash
   dotman-rs init --defaults
   ```

2. **Create your first backup**:
   ```bash
   dotman-rs backup --name "initial-config" ~/.config/
   ```

3. **Verify the backup**:
   ```bash
   dotman-rs list backups
   ```

### For Returning Users

- **Check status**: `dotman-rs status`
- **List existing backups**: `dotman-rs list backups`
- **View configuration**: `dotman-rs config show`

## Comprehensive Usage Guide

### Command Overview

```text
dotman-rs [OPTIONS] <COMMAND>

Global Options:
  -v, --verbose...             Enable verbose logging (use multiple times for more detail)
  -c, --config <CONFIG>        Custom configuration file path
  -n, --dry-run                Show what would be done without executing
  -f, --force                  Force operations without confirmation
  -i, --interactive            Enable interactive mode
  -C, --directory <DIRECTORY>  Working directory
  -h, --help                   Print help information
  -V, --version                Print version information

Commands:
  init     Initialize dotman configuration
  backup   Backup dotfiles and directories
  restore  Restore dotfiles from backup
  list     List backups, contents, profiles, or configuration
  verify   Verify backup integrity
  clean    Clean up old backups
  config   Configuration management
  profile  Profile management
  status   Show status of dotfiles
  diff     Compare dotfiles with backup
  help     Print detailed help for commands
```

### Initialization

#### Basic Initialization
```bash
# Initialize with default settings
dotman-rs init --defaults

# Initialize with custom backup directory
dotman-rs init --backup-dir ~/my-dotfiles-backup

# Initialize for specific profile
dotman-rs init --profile development
```

#### Advanced Initialization
```bash
# Initialize with custom configuration
dotman-rs init \
    --backup-dir ~/backups/dotfiles \
    --config-dir ~/.config/dotman \
    --profile work \
    --compression \
    --verify-integrity
```

### Backup Operations

#### Basic Backup Commands
```bash
# Backup specific files/directories
dotman-rs backup ~/.bashrc ~/.zshrc ~/.config/nvim/

# Backup with custom name and description
dotman-rs backup \
    --name "daily-backup-$(date +%Y%m%d)" \
    --description "Daily configuration backup" \
    ~/.config/

# Backup current directory
dotman-rs backup .
```

#### Advanced Backup Options
```bash
# Backup with compression and verification
dotman-rs backup \
    --name "compressed-backup" \
    --compress \
    --verify \
    ~/.config/

# Backup with pattern exclusions
dotman-rs backup \
    --exclude "*.log" \
    --exclude "cache/*" \
    --exclude ".git/*" \
    --exclude "node_modules/*" \
    ~/development/

# Backup hidden files explicitly
dotman-rs backup \
    --include-hidden \
    --include ".*" \
    ~/

# Encrypted backup (when encryption is implemented)
dotman-rs backup \
    --encrypt \
    --name "secure-backup" \
    ~/sensitive-configs/
```

#### Profile-Based Backups
```bash
# Create work environment backup
dotman-rs backup --profile work ~/work-configs/

# Create development environment backup
dotman-rs backup --profile development ~/.config/

# Create server environment backup
dotman-rs backup --profile server /etc/ ~/.ssh/
```

### Restore Operations

```bash
# List available backups first
dotman-rs list backups

# Restore from specific backup (implementation in progress)
dotman-rs restore --help

# Dry-run restore to see what would be restored
dotman-rs --dry-run restore <backup-id>
```

### Listing and Inspection

#### List Backups
```bash
# List all available backups
dotman-rs list backups

# List backups with detailed information
dotman-rs -v list backups

# List backups for specific profile
dotman-rs --profile work list backups
```

#### Inspect Backup Contents
```bash
# List contents of a specific backup
dotman-rs list contents <backup-id>

# List contents with file details
dotman-rs -v list contents <backup-id>
```

#### List Profiles and Configuration
```bash
# List all profiles
dotman-rs list profiles

# Show current configuration
dotman-rs list config

# Show configuration in detail
dotman-rs -v list config
```

### Configuration Management

#### View and Modify Configuration
```bash
# Show current configuration
dotman-rs config show

# Show specific configuration value
dotman-rs config get backup_dir
dotman-rs config get max_backup_versions

# Set configuration values
dotman-rs config set max_backup_versions 10
dotman-rs config set log_level debug
dotman-rs config set verify_integrity true

# Edit configuration file directly
dotman-rs config edit
```

#### Configuration Validation and Maintenance
```bash
# Validate current configuration
dotman-rs config validate

# Reset configuration to defaults
dotman-rs config reset

# Reset with confirmation
dotman-rs config reset --force
```

### Profile Management

```bash
# See all profile management options
dotman-rs profile --help

# Create new profile
dotman-rs profile create work

# Switch between profiles
dotman-rs profile switch development

# List profile details
dotman-rs profile list --verbose
```

### Verification and Maintenance

#### Integrity Verification
```bash
# Verify specific backup
dotman-rs verify <backup-id>

# Verify all backups
dotman-rs verify --all

# Verify with detailed output
dotman-rs -v verify <backup-id>
```

#### Cleanup Operations
```bash
# Clean up old backups (interactive)
dotman-rs clean

# Clean up automatically based on retention policy
dotman-rs clean --auto

# Clean up backups older than 30 days
dotman-rs clean --older-than 30d

# Force cleanup without confirmation
dotman-rs clean --force
```

#### Status and Comparison
```bash
# Show overall status
dotman-rs status

# Show detailed status
dotman-rs -v status

# Compare files with backup
dotman-rs diff <backup-id>

# Compare specific files
dotman-rs diff <backup-id> ~/.bashrc ~/.zshrc
```

## Configuration

### Configuration File Location

Default configuration locations:
- **Linux/macOS**: `~/.config/dotman/config.toml`
- **Windows**: `%APPDATA%\dotman\config.toml`

### Configuration Structure

```toml
# Basic settings
backup_dir = "/home/user/.local/share/dotman/backups"
config_dir = "/home/user/.config/dotman"
max_backup_versions = 5
log_level = "info"

# File handling
include_patterns = [".*"]
exclude_patterns = [".git/*", "*.log", "cache/*", "node_modules/*"]
follow_symlinks = true
preserve_permissions = true
create_backups = true
verify_integrity = true

# Operation mode
operation_mode = "Default"  # Options: Default, Aggressive, Conservative

# Compression settings
[compression]
enabled = false
level = 6  # 1-9, higher = better compression but slower
algorithm = "Gzip"  # Options: Gzip, Zstd, Lz4

# Encryption settings (when implemented)
[encryption]
enabled = false
algorithm = "Aes256Gcm"
kdf = "Argon2id"

# Logging configuration
[logging]
level = "Info"  # Options: Trace, Debug, Info, Warn, Error
log_to_file = false
structured = false
```

### Environment Variables

```bash
# Override default configuration file
export DOTMAN_CONFIG_PATH="/path/to/custom/config.toml"

# Override backup directory
export DOTMAN_BACKUP_DIR="/path/to/backups"

# Set log level
export RUST_LOG=dotman_rs=debug
```

## Advanced Usage

### Pattern Matching

#### Include Patterns
```bash
# Include specific file types
dotman-rs backup --include "*.conf" --include "*.toml" ~/.config/

# Include specific directories
dotman-rs backup --include "nvim/*" --include "tmux/*" ~/.config/

# Include hidden files
dotman-rs backup --include ".*" ~/
```

#### Exclude Patterns
```bash
# Exclude common unnecessary files
dotman-rs backup \
    --exclude "*.log" \
    --exclude "*.tmp" \
    --exclude "cache/*" \
    --exclude ".git/*" \
    --exclude "node_modules/*" \
    ~/development/

# Complex exclusion patterns
dotman-rs backup \
    --exclude "**/*.pyc" \
    --exclude "**/venv/*" \
    --exclude "**/target/*" \
    ~/projects/
```

### Dry-Run Mode

Test operations without making changes:

```bash
# See what would be backed up
dotman-rs --dry-run backup ~/.config/

# See what would be restored
dotman-rs --dry-run restore <backup-id>

# See what would be cleaned
dotman-rs --dry-run clean --older-than 30d
```

### Verbose Logging

```bash
# Basic verbose output
dotman-rs -v backup ~/.config/

# Very verbose output
dotman-rs -vv backup ~/.config/

# Maximum verbosity
dotman-rs -vvv backup ~/.config/
```

### Scripting and Automation

#### Backup Scripts
```bash
#!/bin/bash
# Daily backup script

DATE=$(date +%Y%m%d)
BACKUP_NAME="daily-$DATE"

dotman-rs backup \
    --name "$BACKUP_NAME" \
    --description "Automated daily backup" \
    --compress \
    --verify \
    ~/.config/ \
    ~/.bashrc \
    ~/.zshrc

# Clean up old backups
dotman-rs clean --older-than 7d --force
```

#### Cron Integration
```bash
# Add to crontab for daily backups at 2 AM
0 2 * * * /usr/local/bin/dotman-rs backup --name "auto-$(date +\%Y\%m\%d)" ~/.config/
```

## Use Cases and Scenarios

### Scenario 1: New User Setting Up Dotfiles Management

**Goal**: Start managing dotfiles for the first time

```bash
# 1. Initialize dotman
dotman-rs init --defaults

# 2. Create initial backup of important configs
dotman-rs backup \
    --name "initial-setup" \
    --description "First backup of my configurations" \
    ~/.bashrc ~/.zshrc ~/.gitconfig ~/.config/

# 3. Verify the backup
dotman-rs list backups
dotman-rs verify initial-setup
```

### Scenario 2: Developer with Multiple Environments

**Goal**: Manage different configurations for work, personal, and server environments

```bash
# 1. Create profiles for different environments
dotman-rs profile create work
dotman-rs profile create personal
dotman-rs profile create server

# 2. Backup work configuration
dotman-rs --profile work backup \
    --name "work-config" \
    ~/work-configs/ ~/.ssh/config.work

# 3. Backup personal configuration
dotman-rs --profile personal backup \
    --name "personal-config" \
    ~/.config/ ~/.bashrc ~/.zshrc

# 4. Switch between profiles as needed
dotman-rs profile switch work
dotman-rs status
```

### Scenario 3: System Administrator Managing Multiple Servers

**Goal**: Backup and synchronize server configurations

```bash
# 1. Create server-specific profile
dotman-rs profile create server-prod

# 2. Backup critical server configs
dotman-rs --profile server-prod backup \
    --name "server-$(hostname)-$(date +%Y%m%d)" \
    --exclude "*.log" \
    --exclude "cache/*" \
    /etc/nginx/ \
    /etc/systemd/ \
    ~/.ssh/

# 3. Verify integrity
dotman-rs verify --all

# 4. Regular cleanup
dotman-rs clean --older-than 30d
```

### Scenario 4: Returning User After System Reinstall

**Goal**: Restore previous configuration on new system

```bash
# 1. Initialize dotman on new system
dotman-rs init --backup-dir /path/to/existing/backups

# 2. List available backups
dotman-rs list backups

# 3. Inspect backup contents
dotman-rs list contents latest-backup

# 4. Restore configuration (when implemented)
dotman-rs restore latest-backup

# 5. Verify restoration
dotman-rs status
```

### Scenario 5: Power User with Complex Setup

**Goal**: Advanced configuration with encryption, compression, and automation

```bash
# 1. Initialize with advanced settings
dotman-rs init \
    --backup-dir ~/secure-backups \
    --compression \
    --verify-integrity

# 2. Configure advanced settings
dotman-rs config set max_backup_versions 20
dotman-rs config set compression.level 9
dotman-rs config set verify_integrity true

# 3. Create comprehensive backup with complex patterns
dotman-rs backup \
    --name "comprehensive-$(date +%Y%m%d)" \
    --compress \
    --verify \
    --include "*.conf" \
    --include "*.toml" \
    --include "*.yaml" \
    --exclude "*.log" \
    --exclude "cache/*" \
    --exclude ".git/*" \
    --exclude "node_modules/*" \
    --exclude "target/*" \
    ~/

# 4. Set up automated cleanup
dotman-rs clean --older-than 90d --force
```

## Troubleshooting

### Common Issues

#### Permission Errors
```bash
# If you encounter permission errors:
sudo dotman-rs backup /etc/

# Or use with privilege detection:
dotman-rs backup --preserve-permissions /etc/
```

#### Large File Sets
```bash
# For large directories, use exclusions:
dotman-rs backup \
    --exclude "node_modules/*" \
    --exclude "target/*" \
    --exclude ".git/*" \
    ~/development/

# Use compression for large backups:
dotman-rs backup --compress ~/large-directory/
```

#### Configuration Issues
```bash
# Validate configuration:
dotman-rs config validate

# Reset to defaults if corrupted:
dotman-rs config reset

# Check configuration location:
dotman-rs config show | head -5
```

### Debug Mode

```bash
# Enable debug logging:
RUST_LOG=dotman_rs=debug dotman-rs backup ~/.config/

# Maximum verbosity:
dotman-rs -vvv backup ~/.config/
```

### Getting Help

```bash
# General help:
dotman-rs --help

# Command-specific help:
dotman-rs backup --help
dotman-rs config --help

# Show version and build info:
dotman-rs --version
```

## Development

### Building from Source

```bash
# Debug build:
cargo build

# Release build:
cargo build --release

# Run tests:
cargo test

# Run with features:
cargo build --features compression,encryption
```

### Running Tests

```bash
# Run all tests:
cargo test

# Run specific test:
cargo test test_backup_restore

# Run with output:
cargo test -- --nocapture

# Run integration tests:
cargo test --test integration
```

### Contributing

1. Fork the repository
2. Create a feature branch: `git checkout -b feature-name`
3. Make changes and add tests
4. Run tests: `cargo test`
5. Run clippy: `cargo clippy`
6. Format code: `cargo fmt`
7. Commit changes: `git commit -am 'Add feature'`
8. Push to branch: `git push origin feature-name`
9. Submit a pull request

## Dependencies

### Runtime Dependencies
- **Rust 1.70+**: Modern Rust with async/await support
- **Operating System**: Linux (primary), macOS, Windows (limited Unix features)

### Key Libraries
- `tokio`: Async runtime for high-performance I/O
- `serde`: Serialization for configuration and metadata
- `blake3`: Fast cryptographic hashing
- `clap`: Command-line argument parsing
- `walkdir`: Efficient directory traversal
- `nix`: Unix system calls (Linux/macOS)

## License

This project is licensed under the [MIT License](LICENSE).

## Roadmap

### Current Status (v0.1.0)
- Core backup functionality
- Configuration management system
- Profile support
- Transaction safety with rollback
- Compression support
- Integrity verification
- Symlink handling
- Pattern matching (include/exclude)
- Progress reporting
- Comprehensive test coverage

### In Development
- Complete restore functionality
- Encryption implementation
- Enhanced backup listing

### Planned Features
- Remote backup support (cloud storage)
- Incremental backups
- Backup scheduling
- Web UI for management
- Configuration templating
- Automatic dotfile detection
- Git repository integration
- Performance optimizations
- Cross-platform compatibility improvements
