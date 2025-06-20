# dotman-rs

A high-performance, cross-platform dotfiles and configuration backup manager written in Rust.

## Overview

dotman-rs is a robust tool for backing up, versioning, and restoring your dotfiles and configuration files. Built with performance and reliability in mind, it provides comprehensive backup management with advanced features like pattern-based inclusion/exclusion, conflict resolution, and integrity verification.

## Features

### Core Functionality
- [x] Fast, concurrent file operations
- [x] Pattern-based file inclusion/exclusion
- [x] Multiple backup versions with automatic cleanup
- [x] Comprehensive conflict resolution strategies
- [x] Backup integrity verification
- [x] Symlink preservation and handling
- [x] Permission preservation (Unix systems)
- [x] Dry-run mode for safe testing
- [x] Progress reporting with detailed feedback

### Advanced Features
- [x] Profile-based configuration management
- [x] Incremental backup support
- [x] Compression support (optional)
- [x] Transaction-based operations with rollback
- [x] Detailed logging and error reporting
- [x] Cross-platform compatibility (Linux, macOS, Windows)

## Installation

### From Source
```bash
git clone https://github.com/yourusername/dotman-rs.git
cd dotman-rs
cargo build --release
sudo cp target/release/dotman-rs /usr/local/bin/
```

### Using Cargo
```bash
cargo install dotman-rs
```

## Quick Start

### Initialize dotman
```bash
# Initialize dotman in your home directory
dotman-rs init

# Or initialize with a specific profile
dotman-rs init --profile work
```

### Basic Backup Operations
```bash
# Backup specific files
dotman-rs backup ~/.bashrc ~/.vimrc ~/.gitconfig

# Backup entire directories
dotman-rs backup ~/.config/nvim ~/.ssh

# Backup with a description
dotman-rs backup ~/.dotfiles --description "Pre-migration backup"

# Dry run to see what would be backed up
dotman-rs backup ~/.config --dry-run
```

### Package-Based Backups
dotman-rs supports organized package-based backups, allowing you to define and backup individual application configurations:

#### Managing Package Configurations
```bash
# List available package configurations
dotman-rs list packages

# Add a new package configuration
dotman-rs package add nvim ~/.config/nvim --description "Neovim configuration files"
dotman-rs package add zsh ~/.config/zsh ~/.zshrc --description "Zsh configuration files"
dotman-rs package add kitty ~/.config/kitty --description "Kitty terminal configuration"

# Add package with exclude/include patterns
dotman-rs package add nvim ~/.config/nvim --description "Neovim config" \
  --exclude "*.log,cache/*" --include "*.lua,*.vim"

# Show package details
dotman-rs package show nvim

# Edit existing package configuration
dotman-rs package edit nvim --description "Updated Neovim configuration" \
  --add-paths ~/.local/share/nvim/site \
  --add-exclude "*.tmp"

# Remove package configuration
dotman-rs package remove nvim
```

#### Using Package-Based Backups
```bash
# Backup individual packages (after defining them)
dotman-rs backup --package nvim
dotman-rs backup --package zsh
dotman-rs backup --package kitty

# Backup with custom name and description
dotman-rs backup --package nvim --name "nvim-pre-update" --description "Before updating to nvim 0.10"

# Combine package backup with additional paths
dotman-rs backup --package zsh ~/.local/share/zsh

# Package backup with additional exclusions
dotman-rs backup --package nvim --exclude "*.log,cache/*"
```

#### Example Package Configurations
Here are some common package configurations you can create:

```bash
# Neovim configuration
dotman-rs package add nvim ~/.config/nvim --description "Neovim configuration files"

# Zsh configuration
dotman-rs package add zsh ~/.config/zsh ~/.zshrc --description "Zsh configuration files"

# Kitty terminal configuration
dotman-rs package add kitty ~/.config/kitty --description "Kitty terminal configuration"

# Tmux configuration
dotman-rs package add tmux ~/.tmux.conf --description "Tmux configuration"

# Git configuration
dotman-rs package add git ~/.gitconfig ~/.gitignore_global --description "Git configuration files"

# VS Code configuration
dotman-rs package add vscode ~/.config/Code/User --description "VS Code settings and extensions" \
  --exclude "logs/*,CachedExtensions/*"
```

#### Individual Package Restore
```bash
# Restore specific package backups
dotman-rs restore package-nvim-20240101 
dotman-rs restore package-zsh-20240101

# Restore package to different location
dotman-rs restore package-nvim-20240101 --target-dir ~/backup-configs/

# Restore specific files from package backup
dotman-rs restore package-nvim-20240101 ~/.config/nvim/init.lua
```

### Package Management Commands

```bash
# Package management
dotman-rs package list                              # List all defined packages
dotman-rs package add <name> <paths...>            # Define a new package
dotman-rs package show <name>                      # Show package details
dotman-rs package edit <name> [options]            # Modify package configuration
dotman-rs package remove <name>                    # Remove package definition

# Package backup/restore
dotman-rs backup --package <name>                  # Backup a defined package
dotman-rs restore <backup-name>                    # Restore package backup
dotman-rs list packages                            # List package configurations
```

### List and Manage Backups
```bash
# List all backups
dotman-rs list backups

# Show detailed backup information
dotman-rs show backup-12345678

# Verify backup integrity
dotman-rs verify backup-12345678

# Clean up old backups (keeps last N versions)
dotman-rs cleanup
```

### Restore Operations
```bash
# Restore specific files from a backup
dotman-rs restore backup-12345678 ~/.bashrc ~/.vimrc

# Restore entire backup
dotman-rs restore backup-12345678

# Restore with conflict resolution
dotman-rs restore backup-12345678 --conflict-resolution overwrite
dotman-rs restore backup-12345678 --conflict-resolution backup
dotman-rs restore backup-12345678 --conflict-resolution skip
```

## Configuration

### Configuration File Location
- Linux/macOS: `~/.config/dotman/config.toml`
- Windows: `%APPDATA%\dotman\config.toml`

### Example Configuration
```toml
backup_dir = "/home/user/.dotman/backups"
config_dir = "/home/user/.config/dotman"

# File patterns to include (glob patterns)
include_patterns = [
    "*",              # Include all files by default
    ".*",             # Include dotfiles
]

# File patterns to exclude
exclude_patterns = [
    ".git/*",         # Git repositories
    "*.log",          # Log files
    "*.tmp",          # Temporary files
    "target/*",       # Rust build artifacts
    "node_modules/*", # Node.js dependencies
    "__pycache__/*",  # Python cache
    "*.pyc",          # Python bytecode
]

# Backup behavior
follow_symlinks = true
preserve_permissions = true
verify_integrity = true
max_backup_versions = 5

# Compression (optional)
enable_compression = false
compression_level = 6

# Logging
log_level = "info"
```

### Profile Management
```bash
# Create a new profile
dotman-rs profile create work --description "Work configuration"

# Switch between profiles
dotman-rs profile use work
dotman-rs profile use personal

# List profiles
dotman-rs profile list

# Show current profile
dotman-rs profile show
```

## Advanced Usage

### Pattern-Based Backup
```bash
# Include only specific file types
dotman-rs backup ~/.config --include "*.conf,*.json,*.yaml"

# Exclude specific directories
dotman-rs backup ~/.config --exclude "cache/*,logs/*"

# Complex pattern example
dotman-rs backup ~/Projects \
  --include "*.rs,*.toml,Cargo.*,README.*" \
  --exclude "target/*,*.lock"
```

### Transaction Management
```bash
# Start a transaction for atomic operations
dotman-rs transaction begin "Major config update"

# Perform multiple operations
dotman-rs backup ~/.config/nvim
dotman-rs backup ~/.config/tmux

# Commit the transaction
dotman-rs transaction commit

# Or rollback if something went wrong
dotman-rs transaction rollback
```

### Backup Verification
```bash
# Verify a specific backup
dotman-rs verify backup-12345678

# Verify all backups
dotman-rs verify --all

# Check backup integrity with detailed output
dotman-rs verify backup-12345678 --verbose
```

### Monitoring and Reporting
```bash
# Show backup statistics
dotman-rs stats

# Generate backup report
dotman-rs report --format json > backup-report.json

# Monitor backup status
dotman-rs status
```

## Command Reference

### Global Options
```
-v, --verbose      Enable verbose output
-q, --quiet        Suppress non-error output
    --dry-run      Show what would be done without executing
-f, --force        Force operation without confirmation
    --config       Specify custom config file path
    --profile      Specify profile to use
```

### Commands

#### `

## Individual Package Management

Dotman-rs supports individual package backup and restore operations, allowing you to manage specific configurations independently. This is particularly useful for backing up and restoring individual applications like `kitty`, `nvim`, `zsh`, etc.

### Creating Package Configurations

Define packages with custom names and paths:

```bash
# Create a package for Neovim configuration
dotman-rs package add nvim ~/.config/nvim --description "Neovim configuration"

# Create a package for Kitty terminal
dotman-rs package add kitty ~/.config/kitty --description "Kitty terminal configuration"

# Create a package for Zsh configuration (multiple paths)
dotman-rs package add zsh ~/.config/zsh ~/.zshrc --description "Zsh shell configuration"
```

### Individual Package Backup

Backup specific packages independently:

```bash
# Backup only nvim configuration
dotman-rs backup --package nvim

# Backup kitty configuration
dotman-rs backup --package kitty

# Backup with custom description
dotman-rs backup --package zsh --description "Pre-update backup"
```

Package backups are stored with descriptive names like `package-nvim-{uuid}` for easy identification.

### Change Detection and Status

Check what has changed since your last backup:

```bash
# Check status of nvim package
dotman-rs status --package nvim

# Check status with detailed output
dotman-rs status --package kitty --detailed

# Check only changed files
dotman-rs status --package zsh --changed-only
```

The status command shows:
- **[=]** Unchanged files
- **[M]** Modified files (different from backup)
- **[+]** New files (not in backup)
- **[-]** Missing files (in backup but not current)

### Compare with Backup (Diff)

View detailed differences between current files and backup:

```bash
# Show differences for nvim package
dotman-rs diff package-nvim-{backup-id} --package nvim

# Show differences with timestamps
dotman-rs diff package-kitty-{backup-id} --package kitty --show-timestamps

# Show all files including identical ones
dotman-rs diff package-zsh-{backup-id} --package zsh --show-identical
```

### Individual Package Restore

Restore specific packages from backup:

```bash
# Restore nvim configuration to original location
dotman-rs restore package-nvim-{backup-id} --package nvim --in-place

# Restore to a different location for testing
dotman-rs restore package-kitty-{backup-id} --package kitty --target ~/test-restore

# Restore with overwrite protection
dotman-rs restore package-zsh-{backup-id} --package zsh --in-place --backup-existing

# Force restore (overwrite existing files)
dotman-rs restore package-nvim-{backup-id} --package nvim --in-place --overwrite
```

### Package Management Commands

```bash
# List all package configurations
dotman-rs package list

# Show details of a specific package
dotman-rs package show nvim

# Edit package configuration (add/remove paths)
dotman-rs package edit nvim --add-paths ~/.config/nvim/after

# Remove a package configuration
dotman-rs package remove nvim
```

### Workflow Example

Here's a typical workflow for managing individual packages:

```bash
# 1. Create package configurations
dotman-rs package add nvim ~/.config/nvim --description "Neovim configuration"
dotman-rs package add kitty ~/.config/kitty --description "Kitty terminal"

# 2. Initial backup
dotman-rs backup --package nvim
dotman-rs backup --package kitty

# 3. Make changes to your configurations...
# (edit files, install plugins, etc.)

# 4. Check what changed
dotman-rs status --package nvim
dotman-rs status --package kitty --changed-only

# 5. Create new backup if satisfied with changes
dotman-rs backup --package nvim --description "Added new plugins"

# 6. Or restore previous version if needed
dotman-rs restore package-nvim-{previous-backup-id} --package nvim --in-place --overwrite
```

### Benefits of Individual Package Management

- **Granular Control**: Backup and restore specific applications independently
- **Change Tracking**: See exactly what changed in each package since last backup
- **Selective Restore**: Restore only the configurations you need
- **Organized Backups**: Package backups are clearly labeled and easy to identify
- **Conflict Prevention**: Test configurations in isolated directories before applying
- **Version History**: Maintain multiple backup versions for each package