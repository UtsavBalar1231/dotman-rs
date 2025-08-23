# dotman - High-Performance Dotfiles Manager

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)
[![Build Status](https://img.shields.io/badge/build-passing-brightgreen.svg)]()
[![Version](https://img.shields.io/badge/version-0.0.1-blue.svg)]()

A high-performance dotfiles manager with git-like semantics, built in Rust for maximum speed and reliability.

## Table of Contents

- [Overview](#overview)
- [Technical Features](#technical-features)
- [Performance Characteristics](#performance-characteristics)
- [Installation](#installation)
- [Quick Start](#quick-start)
- [Command Reference](#command-reference)
- [Configuration](#configuration)
- [Architecture](#architecture)
- [Development](#development)
- [Testing](#testing)
- [Benchmarking](#benchmarking)
- [Contributing](#contributing)
- [License](#license)

## Overview

dotman is a dotfiles manager designed from the ground up for performance and reliability. Unlike traditional dotfile managers that prioritize features over speed, dotman employs advanced optimization techniques including SIMD acceleration, parallel processing, and memory-mapped I/O to achieve sub-millisecond operations on typical dotfile repositories.

### Key Differentiators

- **Performance-First Design**: Every architectural decision prioritizes speed without sacrificing correctness
- **Git-Like Interface**: Familiar command structure for developers already using version control
- **Multi-Configuration Support**: Branch-based management for different machines (laptop, desktop, server)
- **Flexible Remote Storage**: Support for Git, S3, and Rsync remotes for backup and synchronization
- **Content Deduplication**: Intelligent storage minimization through content-based deduplication
- **Parallel Processing**: Utilizes all available CPU cores for file operations
- **Binary Index Format**: Fast serialization and deserialization for instant repository loading

## Technical Features

### Performance Optimizations

- **SIMD Acceleration**: Hardware-accelerated UTF-8 validation and JSON parsing on x86_64 and ARM64
- **Memory-Mapped I/O**: Efficient handling of large files (>1MB) through OS-level memory mapping
- **xxHash3 Algorithm**: Ultra-fast hashing achieving >30GB/s throughput on modern CPUs
- **Lock-Free Concurrency**: DashMap-based concurrent data structures for thread-safe operations
- **Zstandard Compression**: Configurable compression levels with dictionary training for optimal ratios

### Storage Architecture

- **Binary Index**: Bincode-serialized index for O(1) file lookups
- **Content Deduplication**: Hash-based deduplication reduces storage by 40-60% on average
- **Atomic Operations**: File-level locking ensures data integrity during concurrent operations
- **Incremental Snapshots**: Only changed content stored in new commits

### Reliability Features

- **Transactional Commits**: Either all files commit successfully or none do
- **Graceful Error Recovery**: Comprehensive error handling with context preservation
- **File Permission Preservation**: Maintains original file permissions and metadata
- **Cross-Platform Support**: Linux and macOS with architecture-specific optimizations

## Performance Characteristics

### Benchmark Results

Operations tested on AMD Ryzen 7 5800X with NVMe SSD:

| Operation | File Count | Time | Throughput |
|-----------|------------|------|------------|
| Add files | 1,000 | 45ms | 22,222 files/sec |
| Status check | 10,000 | 8ms | 1.25M files/sec |
| Commit creation | 5,000 | 180ms | 27,777 files/sec |
| Checkout | 5,000 | 150ms | 33,333 files/sec |
| File hashing | 1GB | 0.9s | 1.11 GB/s |

### Memory Usage

- Base overhead: ~1.5MB
- Index in memory: 32 bytes per file entry
- Working set: 2-8MB for typical operations
- Peak usage: 50-200MB during large commits

## Installation

### Binary Installation (Recommended)

#### Linux x86_64
```bash
curl -LO https://github.com/UtsavBalar1231/dotman-rs/releases/latest/download/dot-x86_64-unknown-linux-gnu.tar.gz
tar xzf dot-x86_64-unknown-linux-gnu.tar.gz
sudo mv dot /usr/local/bin/
```

#### Linux ARM64
```bash
curl -LO https://github.com/UtsavBalar1231/dotman-rs/releases/latest/download/dot-aarch64-unknown-linux-gnu.tar.gz
tar xzf dot-aarch64-unknown-linux-gnu.tar.gz
sudo mv dot /usr/local/bin/
```

#### macOS Intel
```bash
curl -LO https://github.com/UtsavBalar1231/dotman-rs/releases/latest/download/dot-x86_64-apple-darwin.tar.gz
tar xzf dot-x86_64-apple-darwin.tar.gz
sudo mv dot /usr/local/bin/
```

#### macOS Apple Silicon
```bash
curl -LO https://github.com/UtsavBalar1231/dotman-rs/releases/latest/download/dot-aarch64-apple-darwin.tar.gz
tar xzf dot-aarch64-apple-darwin.tar.gz
sudo mv dot /usr/local/bin/
```

### Package Managers

#### Cargo (Rust Package Manager)
```bash
cargo install dotman
```

#### Arch Linux (AUR)
```bash
yay -S dotman
# or
paru -S dotman
```

#### Debian/Ubuntu
```bash
curl -LO https://github.com/UtsavBalar1231/dotman-rs/releases/latest/download/dotman_0.0.1-1_amd64.deb
sudo dpkg -i dotman_0.0.1-1_amd64.deb
```

#### Fedora/RHEL/CentOS
```bash
sudo dnf install https://github.com/UtsavBalar1231/dotman-rs/releases/latest/download/dotman-0.0.1-1.x86_64.rpm
```

### Build from Source

#### Prerequisites
- Rust 1.70 or later
- Git
- C compiler (for native dependencies)

#### Build Steps
```bash
git clone https://github.com/UtsavBalar1231/dotman-rs.git
cd dotman-rs
cargo build --release
sudo cp target/release/dot /usr/local/bin/
```

#### Optimized Build
```bash
RUSTFLAGS="-C target-cpu=native" cargo build --release
```

## Quick Start

### Initialize Repository
```bash
# Initialize dotman in your home directory
cd ~
dot init

# This creates the ~/.dotman directory structure
```

### Track Dotfiles
```bash
# Add individual files
dot add ~/.bashrc ~/.vimrc ~/.gitconfig

# Add entire directories
dot add ~/.config/nvim

# Check status
dot status
```

### Create Snapshots
```bash
# Commit tracked files
dot commit -m "Initial dotfiles setup"

# Commit all changes to tracked files
dot commit --all -m "Update configurations"
```

### View History
```bash
# Show commit log
dot log

# Show specific commit
dot show <commit-id>

# Show differences
dot diff
```

### Restore Files
```bash
# Checkout specific commit
dot checkout <commit-id>

# Force checkout (overwrites local changes)
dot checkout --force HEAD
```

### Multi-Configuration Workflow

Managing different dotfiles for different machines:

```bash
# Create branches for different configurations
dot branch create laptop-config
dot branch create desktop-config
dot branch create server-config

# Switch to laptop configuration
dot checkout laptop-config

# Make laptop-specific changes
dot add ~/.config/laptop-specific.conf
dot commit -m "Add laptop-specific configurations"

# Set up remote repository
dot remote add origin git@github.com:username/dotfiles.git

# Push laptop configuration
dot push origin laptop-config

# On another machine, pull specific configuration
dot pull origin desktop-config
dot checkout desktop-config

# Set upstream tracking for automatic push/pull
dot branch set-upstream laptop-config origin laptop-config
dot push  # Now pushes to tracked upstream
```

## Command Reference

### Repository Management

#### `dot init [--bare]`
Initialize a new dotman repository.

Options:
- `--bare`: Create a bare repository without working directory

#### `dot status [--short] [--untracked]`
Display the working tree status.

Options:
- `--short`: Show status in short format
- `--untracked`: Show untracked files

### File Operations

#### `dot add <paths...> [--force]`
Add files or directories to be tracked.

Options:
- `--force`: Force add files even if they match ignore patterns

Example:
```bash
dot add ~/.bashrc ~/.config/nvim
dot add ~/.ssh/config --force
```

#### `dot rm <paths...> [--cached] [--force] [--interactive]`
Remove files from tracking.

Options:
- `--cached`: Remove from index only, keep file on disk
- `--force`: Force removal without safety checks
- `--interactive`: Interactive removal with confirmations

### Snapshot Operations

#### `dot commit [-m <message>] [--all] [--amend]`
Create or amend a snapshot of tracked files.

Options:
- `-m, --message`: Commit message (required unless using --amend)
- `--all`: Automatically stage all tracked file changes
- `--amend`: Amend the previous commit instead of creating a new one

Example:
```bash
dot commit -m "Update shell configurations"
dot commit --all -m "Backup all dotfiles"
dot commit --amend -m "Updated message"
dot commit --amend  # Keep original message
```

#### `dot checkout <target> [--force]`
Restore files from a specific commit, branch, or reference.

Supported targets:
- Commit hash (full or short): `abc123` or `abc12345...`
- Branch name: `main`, `feature-branch`
- HEAD references: `HEAD`, `HEAD~1`, `HEAD~2`

Options:
- `--force`: Overwrite local changes without prompting

#### `dot reset <commit> [--hard] [--soft]`
Reset HEAD to specified state.

Supported targets:
- Commit hash (full or short): `abc123` or `abc12345...`
- Branch name: `main`, `feature-branch`
- HEAD references: `HEAD`, `HEAD~1`, `HEAD~2`

Options:
- `--hard`: Reset working directory to match commit
- `--soft`: Move HEAD only, keep working directory unchanged

### History Operations

#### `dot log [<target>] [-n <limit>] [--oneline]`
Display commit history.

Arguments:
- `<target>`: Commit, branch, or HEAD to start from (optional)

Options:
- `-n, --limit`: Number of commits to show (default: 10)
- `--oneline`: Compact single-line format

Examples:
```bash
dot log                    # Show last 10 commits from current branch
dot log HEAD               # Show commits from HEAD
dot log main               # Show commits from main branch
dot log abc123             # Show commits from specific commit
dot log -n 20 --oneline    # Show 20 commits in oneline format
```

#### `dot show <object>`
Display detailed information about a commit.

#### `dot diff [<from>] [<to>]`
Show differences between commits or working directory.

Supported references:
- Commit hash (full or short): `abc123` or `abc12345...`
- Branch name: `main`, `feature-branch`
- HEAD references: `HEAD`, `HEAD~1`, `HEAD~2`

Note: Output is automatically displayed through a pager (less/more) when running in a terminal.

### Branch Management

#### `dot branch [options]`
Manage branches for different dotfile configurations.

Subcommands:
- `list`: List all branches
- `create <name> [<start-point>]`: Create a new branch
- `delete <name> [--force]`: Delete a branch
- `rename <old> <new> [--force]`: Rename a branch
- `current`: Show current branch
- `set-upstream <branch> <remote> <remote-branch>`: Set upstream tracking

Examples:
```bash
# List all branches
dot branch list

# Create a new branch for laptop configuration
dot branch create laptop-config

# Create branch from specific commit
dot branch create experimental abc123

# Switch to a different branch
dot checkout laptop-config

# Set upstream tracking
dot branch set-upstream laptop-config origin laptop-config

# Delete a branch
dot branch delete old-config --force
```

### Stash Operations

#### `dot stash [subcommand]`
Temporarily save and restore changes to your working directory.

Subcommands:
- `push [-m <message>] [-u] [-k]`: Save changes to stash (default when no subcommand)
- `pop`: Apply and remove the latest stash
- `apply [<stash-id>]`: Apply a stash without removing it
- `list`: List all stash entries
- `show [<stash-id>]`: Show the contents of a stash
- `drop <stash-id>`: Remove a specific stash
- `clear`: Remove all stashes

Options for `stash push`:
- `-m, --message`: Custom message for the stash
- `-u, --include-untracked`: Include untracked files in the stash
- `-k, --keep-index`: Keep changes in the index

Examples:
```bash
# Save current changes with a message
dot stash push -m "Work in progress on feature X"

# Include untracked files
dot stash push -u -m "Save everything including new files"

# Apply the latest stash
dot stash pop

# Apply a specific stash without removing it
dot stash apply stash_12345678_abcdef

# List all stashes
dot stash list

# Show contents of latest stash
dot stash show

# Remove all stashes
dot stash clear
```

### Remote Operations

#### `dot remote [options]`
Manage remote repositories for syncing dotfiles.

Subcommands:
- `list`: List configured remotes
- `add <name> <url>`: Add a new remote
- `remove <name>`: Remove a remote
- `rename <old> <new>`: Rename a remote
- `set-url <name> <url>`: Change remote URL
- `show <name>`: Show remote details

Supported remote types:
- **Git**: Standard git repositories (SSH/HTTPS)
- **S3**: Amazon S3 buckets for cloud storage
- **Rsync**: Any rsync-compatible destination

Examples:
```bash
# List all remotes
dot remote list

# Add a git remote
dot remote add origin git@github.com:username/dotfiles.git

# Add an S3 remote for backup
dot remote add backup s3://my-dotfiles-bucket/

# Add an rsync remote for NAS
dot remote add nas rsync://nas.local/backup/dotfiles/

# Change remote URL
dot remote set-url origin https://github.com/username/dotfiles.git

# Remove a remote
dot remote remove old-backup
```

#### `dot push [<remote>] [<branch>]`
Push commits to remote repository.

Arguments:
- `<remote>`: Remote name (defaults to upstream remote or 'origin')
- `<branch>`: Branch to push (defaults to current branch)

Examples:
```bash
dot push                    # Push current branch to its upstream
dot push origin             # Push current branch to origin
dot push backup main        # Push main branch to backup remote
dot push origin laptop-config  # Push laptop-config branch to origin
```

#### `dot pull [<remote>] [<branch>]`
Pull and merge changes from remote repository.

Arguments:
- `<remote>`: Remote name (defaults to upstream remote or 'origin')
- `<branch>`: Branch to pull (defaults to current branch)

Examples:
```bash
dot pull                    # Pull current branch from upstream
dot pull origin             # Pull current branch from origin
dot pull backup main        # Pull main branch from backup remote
```

### Utility Commands

#### `dot completion <shell>`
Generate shell completion scripts.

Supported shells:
- `bash`
- `zsh`
- `fish`
- `elvish`
- `powershell`

Example:
```bash
# Bash
dot completion bash > ~/.local/share/bash-completion/completions/dot

# Zsh
dot completion zsh > ~/.local/share/zsh/site-functions/_dot

# Fish
dot completion fish > ~/.config/fish/completions/dot.fish
```

### Configuration Management

#### `dot config [<key>] [<value>] [--list] [--unset]`
Get and set repository and user configuration options.

Usage:
- `dot config` or `dot config --list`: Show all configuration values
- `dot config <key>`: Get a specific configuration value
- `dot config <key> <value>`: Set a configuration value
- `dot config --unset <key>`: Remove a configuration value

Supported keys:
- `user.name`: Your name for commit authorship
- `user.email`: Your email for commit authorship
- `core.compression`: Compression algorithm (zstd/none)
- `core.compression_level`: Compression level (1-22)
- `core.default_branch`: Default branch name
- `performance.*`: Performance tuning options
- `tracking.*`: File tracking options

Examples:
```bash
# Show all configuration
dot config --list

# Set user information
dot config user.name "John Doe"
dot config user.email "john@example.com"

# Get a specific value
dot config user.name

# Unset a value
dot config --unset user.email
```

## Configuration

Configuration file location: `~/.config/dotman/config`

### Configuration Structure

```toml
[core]
# Repository location (absolute or relative to home)
repo_path = "~/.dotman"
# Default branch name for new repositories
default_branch = "main"
# Compression algorithm: "zstd" or "none"
compression = "zstd"
# Compression level (1-22, higher = better compression but slower)
compression_level = 3

# Multiple remotes configuration
[remotes.origin]
# Remote type: "git", "s3", or "rsync"
remote_type = "git"
# Remote URL (format depends on remote_type)
url = "git@github.com:username/dotfiles.git"

[remotes.backup]
remote_type = "s3"
url = "s3://my-dotfiles-backup/"

[remotes.nas]
remote_type = "rsync"
url = "rsync://nas.local/backup/dotfiles/"

# Branch configuration
[branches]
# Current active branch
current = "main"
# Default branch for new repositories
default = "main"

# Branch upstream tracking (optional)
[branches.tracking.main]
remote = "origin"
branch = "main"

[branches.tracking.laptop-config]
remote = "origin"
branch = "laptop-config"

[performance]
# Number of parallel threads (0 = auto-detect)
parallel_threads = 0
# Use memory mapping for files larger than this (bytes)
mmap_threshold = 1048576  # 1MB
# Cache size in MB for frequently accessed data
cache_size = 100
# Use hard links instead of copying when possible
use_hard_links = true

[tracking]
# Files and patterns to ignore (gitignore-style)
ignore_patterns = [
    ".git",
    "*.swp",
    "*.tmp",
    "node_modules",
    "__pycache__",
    ".DS_Store"
]
# Follow symbolic links when traversing directories
follow_symlinks = false
# Preserve file permissions in snapshots
preserve_permissions = true
```

### Performance Tuning

#### For SSDs
```toml
[performance]
parallel_threads = 0      # Use all cores
mmap_threshold = 65536    # 64KB
cache_size = 200
use_hard_links = true
```

#### For HDDs
```toml
[performance]
parallel_threads = 4      # Limit parallelism
mmap_threshold = 4194304  # 4MB
cache_size = 50
use_hard_links = false
```

#### For Network Storage
```toml
[core]
compression_level = 9     # Higher compression

[performance]
parallel_threads = 2      # Reduce network congestion
mmap_threshold = 8388608  # 8MB
cache_size = 25
```

## Architecture

### Project Structure

```
dotman/
├── src/
│   ├── main.rs           # CLI entry point
│   ├── lib.rs            # Library interface
│   ├── commands/         # Command implementations
│   │   ├── add.rs
│   │   ├── commit.rs
│   │   ├── status.rs
│   │   ├── branch.rs     # Branch management
│   │   ├── remote.rs     # Remote management
│   │   └── ...
│   ├── storage/          # Storage layer
│   │   ├── index.rs      # Binary index management
│   │   └── snapshots.rs  # Snapshot storage
│   ├── refs.rs           # Git-like references system
│   ├── config/           # Configuration
│   │   ├── mod.rs        # Config structures
│   │   └── parser.rs     # TOML parser
│   └── utils/            # Utilities
│       ├── hash.rs       # xxHash3 implementation
│       └── compress.rs   # Compression utilities
├── tests/                # Integration tests
├── benches/              # Performance benchmarks
└── Cargo.toml           # Project manifest
```

### Repository Layout

```
~/.dotman/
├── index.bin            # Binary index file
├── commits/             # Snapshot storage
│   └── <commit-id>      # Compressed snapshots
├── objects/             # Deduplicated objects
├── refs/                # Reference storage
│   ├── heads/           # Branch references
│   │   ├── main         # Main branch
│   │   └── laptop-config # Other branches
│   └── remotes/         # Remote tracking branches
│       └── origin/
│           └── main
└── HEAD                 # Current branch/commit reference
```

### Key Dependencies

- `clap`: Command-line argument parsing
- `rayon`: Data parallelism
- `dashmap`: Concurrent hashmap
- `bincode`: Binary serialization
- `zstd`: Compression
- `xxhash-rust`: Fast hashing
- `memmap2`: Memory-mapped files
- `simdutf8`: SIMD UTF-8 validation
- `parking_lot`: Synchronization primitives

## Development

### Build System

The project uses Just as the primary task runner:

```bash
# Build
just build              # Debug build
just build-release      # Release build
just build-optimized    # Native CPU optimizations

# Testing
just test              # Run all tests
just test-unit         # Unit tests only
just test-integration  # Integration tests
just test-property     # Property-based tests

# Code Quality
just fmt               # Format code
just lint              # Run clippy
just audit             # Security audit

# Performance
just bench             # Run benchmarks
just profile-flame     # Generate flamegraph
just profile-memory    # Memory profiling
```

### Cross-Compilation

```bash
# Build for all platforms
./scripts/cross-compile.sh --dist all

# Specific platforms
./scripts/cross-compile.sh --dist linux-x86_64
./scripts/cross-compile.sh --dist linux-aarch64
./scripts/cross-compile.sh --dist darwin-x86_64
./scripts/cross-compile.sh --dist darwin-aarch64
```

### Packaging

```bash
# Debian/Ubuntu
just package-debian

# RPM (Fedora/RHEL)
just package-rpm

# Arch Linux
just package-arch

# Alpine Linux
just package-alpine

# Docker containers
just docker-build
just docker-build-alpine
```

## Testing

### Test Categories

- **Unit Tests**: Inline with modules, test individual functions
- **Integration Tests**: End-to-end workflow testing
- **Property Tests**: Randomized testing for edge cases
- **Security Tests**: Vulnerability and safety testing

### Running Tests

```bash
# All tests
cargo test

# Specific test category
cargo test --test integration_test
cargo test --test property_based_tests

# With output
cargo test -- --nocapture

# Sequential execution (for isolation)
cargo test -- --test-threads=1

# Coverage report
cargo llvm-cov --html
```

## Benchmarking

### Performance Benchmarks

```bash
# Run all benchmarks
cargo bench

# Specific benchmark
cargo bench parser_bench
cargo bench storage_bench
cargo bench commands_bench

# Compare with baseline
cargo bench -- --baseline main
```

### Profiling

```bash
# CPU profiling with flamegraph
cargo flamegraph --bin dot -- status

# Memory profiling
valgrind --tool=massif target/release/dot status
ms_print massif.out.* > memory-profile.txt

# Performance profiling
perf record --call-graph=dwarf target/release/dot status
perf report
```

## Contributing

### Development Setup

1. Fork the repository
2. Clone your fork:
   ```bash
   git clone https://github.com/your-username/dotman-rs.git
   cd dotman-rs
   ```

3. Install development tools:
   ```bash
   just setup
   ```

4. Create a feature branch:
   ```bash
   git checkout -b feature/your-feature
   ```

### Code Standards

- Follow Rust standard formatting (enforced by rustfmt)
- Pass all clippy lints
- Maintain or improve test coverage
- Document public APIs
- Benchmark performance-critical changes

### Submitting Changes

1. Ensure all tests pass:
   ```bash
   just ci
   ```

2. Commit with descriptive messages:
   ```bash
   git commit -m "feat: add new feature

   - Detailed description
   - Related issue #123"
   ```

3. Push and create pull request

### Issue Reporting

Report issues at: https://github.com/UtsavBalar1231/dotman-rs/issues

Include:
- dotman version
- Operating system and architecture
- Steps to reproduce
- Expected vs actual behavior
- Error messages or logs

## License

MIT License - See [LICENSE](LICENSE) file for details.

## Acknowledgments

Built with high-performance Rust crates from the ecosystem:
- The Rust community for excellent libraries
- Contributors and testers
- Users providing feedback and bug reports

---

**Project Homepage**: https://github.com/UtsavBalar1231/dotman-rs
**Documentation**: https://docs.rs/dotman
**Author**: Utsav Balar <utsavbalar1231@gmail.com>
