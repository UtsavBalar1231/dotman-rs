# dotman

[![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Build Status](https://img.shields.io/github/actions/workflow/status/yourusername/dotman/ci.yml?branch=main)](https://github.com/yourusername/dotman/actions)

High-performance dotfiles manager with Git-like semantics, built in Rust.

## Why dotman?

Dotman brings the power and familiarity of Git to dotfiles management, with a focus on **performance** and **simplicity**. Unlike symlink-based managers, dotman uses content-addressed storage with deduplication, making it fast and storage-efficient. With SIMD acceleration, parallel processing, and memory-mapped I/O, it handles large dotfile collections effortlessly.

- **Git-like workflow** - If you know Git, you know dotman
- **Content deduplication** - Same files stored only once via xxHash3
- **Blazing fast** - SIMD optimizations, parallel operations, binary index
- **Branch-based configs** - Different setups for different machines
- **Git remote support** - Backup to GitHub/GitLab/Bitbucket

## Installation

### From Source (Recommended)
```bash
# Install Rust if needed
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Clone and install
git clone https://github.com/yourusername/dotman.git
cd dotman
cargo install --path .
```

### Installation Script
```bash
# Unix/Linux/macOS
curl -fsSL https://raw.githubusercontent.com/yourusername/dotman/main/install/install.sh | bash

# Or download and run manually
wget https://raw.githubusercontent.com/yourusername/dotman/main/install/install.sh
chmod +x install.sh
./install.sh
```

### Pre-built Binaries

Download from [releases](https://github.com/yourusername/dotman/releases) for your platform:

**Tier 1 Support (Binary Releases Available):**
- Linux x86_64 (glibc) - `dotman-v{version}-x86_64-unknown-linux-gnu.tar.gz`
- Linux x86_64 (musl) - `dotman-v{version}-x86_64-unknown-linux-musl.tar.gz`
- Linux aarch64 (glibc) - `dotman-v{version}-aarch64-unknown-linux-gnu.tar.gz`
- Linux aarch64 (musl) - `dotman-v{version}-aarch64-unknown-linux-musl.tar.gz`

**Tier 2 Support (Source Build Only):**
- macOS (Intel, Apple Silicon)
- Windows (x86_64)

### Package Managers (Planned)

The following package manager integrations are planned but not yet published:
```bash
# Arch Linux (AUR) - Coming soon
# yay -S dotman

# Homebrew (macOS/Linux) - Coming soon
# brew install dotman

# Cargo - Coming soon
# cargo install dotman
```

For now, please use source installation or download pre-built binaries from GitHub releases.

## Quick Start

```bash
# Initialize repository
dot init

# Track dotfiles
dot add ~/.zshrc ~/.vimrc ~/.config/nvim
dot commit -m "Initial dotfiles"

# Create machine-specific branch
dot checkout -b laptop
# OR: dot branch -b laptop
dot add ~/.config/i3
dot commit -m "Add laptop-specific configs"

# Set up remote backup
dot remote add origin git@github.com:yourusername/dotfiles.git
dot push origin main

# Restore on new machine
dot init
dot remote add origin git@github.com:yourusername/dotfiles.git
dot pull origin main
dot checkout main
```

## Core Commands

### Essential Operations
| Command | Description |
|---------|------------|
| `dot init` | Initialize a new repository |
| `dot add <files>` | Track files or directories |
| `dot rm <files>` | Remove files from tracking (never deletes from disk) |
| `dot status` | Show tracked/modified/deleted files |
| `dot commit -m "msg"` | Create a snapshot |
| `dot reset <ref>` | Reset to a specific commit (--hard, --soft, --mixed, --keep) |

### File Management
| Command | Description |
|---------|------------|
| `dot restore -s <commit> <files>` | Restore specific files from a commit |
| `dot clean` | Remove untracked files (-n for dry-run, -f to force) |
| `dot diff [from] [to]` | Show differences (working vs index, commit vs working, or commit vs commit) |

### Branching & History
| Command | Description |
|---------|------------|
| `dot branch` | List all branches |
| `dot branch create <name>` | Create a new branch |
| `dot branch delete <name>` | Delete a branch (-f to force) |
| `dot branch checkout <name>` | Switch to a branch |
| `dot branch rename <old> <new>` | Rename a branch |
| `dot branch set-upstream <name> <remote> <branch>` | Set upstream tracking |
| `dot branch unset-upstream <name>` | Remove upstream tracking |
| `dot branch -b <name> [start]` | Create and switch to new branch (shorthand) |
| `dot checkout <ref>` | Restore files from snapshot (branch or commit) |
| `dot checkout -b <name> [start]` | Create and switch to new branch |
| `dot log` | View commit history (--oneline, -n <limit>) |
| `dot show <object>` | Display commit details |
| `dot reflog` | View HEAD movement history (--all for all refs) |

### Tags
| Command | Description |
|---------|------------|
| `dot tag create <name> [commit]` | Create a tag (defaults to HEAD) |
| `dot tag list` | List all tags |
| `dot tag delete <name>` | Delete a tag (-f to force) |
| `dot tag show <name>` | Display tag details |

### Remote Operations
| Command | Description |
|---------|------------|
| `dot remote add <name> <url>` | Add remote repository |
| `dot remote list` | List all remotes |
| `dot remote remove <name>` | Remove a remote |
| `dot remote set-url <name> <url>` | Update remote URL |
| `dot remote show <name>` | Display remote details |
| `dot remote rename <old> <new>` | Rename a remote |
| `dot push [remote] [branch]` | Push changes (--force, --force-with-lease, --dry-run, --tags, -u) |
| `dot pull [remote] [branch]` | Pull and merge changes (--rebase, --no-ff, --squash) |
| `dot fetch [remote]` | Download remote changes (--all, --tags) |

### Advanced Operations
| Command | Description |
|---------|------------|
| `dot stash push` | Save current changes (-m for message, -u for untracked, -k to keep index) |
| `dot stash pop` | Apply and remove latest stash |
| `dot stash apply [stash]` | Apply stash without removing |
| `dot stash list` | List all stashes |
| `dot stash show [stash]` | Show stash contents |
| `dot stash drop <stash>` | Delete a specific stash |
| `dot stash clear` | Delete all stashes |
| `dot merge <branch>` | Merge branches (--no-ff, --squash, -m for message) |
| `dot revert <commit>` | Create a revert commit (--no-edit, -f to force) |
| `dot import <source>` | Import from git repos (--track, -f, --dry-run, -y) |

### Utility Commands
| Command | Description |
|---------|------------|
| `dot config [key] [value]` | Get/set configuration (--unset, -l to list) |
| `dot completion <shell>` | Generate shell completions (bash, zsh, fish, powershell, elvish) |

## Configuration

Dotman uses TOML configuration at `~/.config/dotman/config`:

```toml
[core]
compression_level = 3        # 1-22, higher = better compression (default: 3)
pager = "less"              # Optional pager for output

[user]
name = "Your Name"
email = "you@example.com"

[performance]
parallel_threads = 0         # 0 = auto-detect (max 8 cores)
mmap_threshold = 1048576    # Use mmap for files > 1MB
use_hard_links = true       # Use hard links for deduplication

[tracking]
ignore_patterns = [".git", "*.swp", "*.tmp", "node_modules", "__pycache__"]
follow_symlinks = false
preserve_permissions = true
large_file_threshold = 104857600  # 100MB
```

**Environment Variables:**
- `DOTMAN_CONFIG_PATH` - Override config file location
- `DOTMAN_REPO_PATH` - Override repository location (default: `~/.dotman`)

## Architecture

Dotman uses a **trait-based architecture** with content-addressed storage for maximum performance and code reuse:

```
┌─────────────────────────────────────────┐
│            CLI (clap)                   │
├─────────────────────────────────────────┤
│     CommandContext Trait                │  ← Shared command logic
│     RemoteOperations Trait              │  ← Remote sync operations
├─────────────────────────────────────────┤
│     Dual Index System                   │
│     • Index (HashMap, single-thread)    │  ← O(1) lookups
│     • ConcurrentIndex (DashMap)         │  ← Lock-free parallel ops
├─────────────────────────────────────────┤
│     Content Store (xxHash3)             │  ← Deduplication
│     Snapshots (Zstd compressed)         │  ← Commit storage
├─────────────────────────────────────────┤
│     SIMD UTF-8 & JSON (x86_64/aarch64)  │  ← Hardware acceleration
│     Memory-mapped I/O (>1MB threshold)  │  ← Large file handling
│     Parallel Processing (rayon)         │  ← Multi-core utilization
│     Hash Caching (size + mtime)         │  ← Avoid re-hashing
└─────────────────────────────────────────┘
```

### Repository Structure
```
~/.dotman/
├── index.bin          # Binary index (bincode-serialized)
├── commits/           # Snapshot storage (zstd-compressed)
├── objects/           # Deduplicated content (content-addressed)
├── refs/              # Branches and tags
│   ├── heads/         # Local branches
│   └── tags/          # Tags
├── logs/HEAD          # Reflog for recovery
└── HEAD              # Current reference
```

## Development

### Building
```bash
# Using Just (recommended task runner)
just build               # Debug build
just build-release       # Release build with optimizations
just build-optimized     # Native CPU optimizations

# Or using Cargo directly
cargo build
cargo build --release
```

### Testing
```bash
# Using Just
just test               # Run all tests
just test-verbose       # With output
just test-unit          # Library tests only
just ci                 # Full CI suite (fmt-check, lint, test, security, docs)

# Or using Cargo
cargo test --all-features
```

### Code Quality
```bash
# Using Just
just fmt                # Format code
just lint               # Clippy with strict warnings
just fix                # Auto-fix issues

# Or using Cargo
cargo fmt
cargo clippy
```

### Development Setup
```bash
# Install all development tools
just setup

# Watch for changes and rebuild
just watch              # Auto-rebuild on changes
just watch-test         # Auto-test on changes
```

### Running dotman
```bash
# Debug build
cargo run -- <command>

# Release build
cargo run --release -- <command>

# Or after building
./target/release/dot <command>

# With verbose output
RUST_LOG=debug dot status
```

## Performance

Dotman is optimized for speed at every level with **concrete, measurable characteristics**:

### Hash & Deduplication
- **XXH3 hashing**: ~30GB/s on modern CPUs (non-cryptographic, deterministic)
- **Content deduplication**: Identical files stored once via content addressing
- **Hash caching**: Stores hash with size + mtime to avoid re-hashing unchanged files
  - Cache hit: size and mtime unchanged → reuse hash (instant)
  - Cache miss: file modified → recompute hash

### I/O Optimization
- **Memory-mapped I/O**: Automatic for files >1MB (configurable threshold)
  - Small files (<1MB): Standard read into memory
  - Large files (≥1MB): Memory-mapped for efficiency
- **Parallel file operations**: Rayon threadpool with work-stealing
  - Default: min(CPU count, 8) threads
  - Configurable via `performance.parallel_threads`

### Concurrency
- **Lock-free concurrent access**: DashMap for parallel index operations
- **Dual index system**:
  - `Index` (HashMap) for single-threaded operations
  - `ConcurrentIndex` (DashMap) for parallel operations
- **Singleton thread pool**: Initialized once, reused across operations

### Compression
- **Zstandard compression**: Configurable levels 1-22 (default: 3)
  - Level 1: Fastest compression
  - Level 3: Balanced (default)
  - Level 22: Maximum compression
- **Dictionary training**: Optional 10-30% better compression for similar files

### Serialization
- **Binary index**: Bincode v2.0 (faster than JSON/TOML)
- **SIMD acceleration**: UTF-8 validation and JSON parsing (x86_64/aarch64 only)

## Contributing

Contributions welcome! Please ensure:
- Tests pass: `just test` or `cargo test`
- Code is formatted: `just fmt` or `cargo fmt`
- Clippy is happy: `just lint` or `cargo clippy`

### Development Workflow
1. Fork the repository
2. Create a feature branch: `git checkout -b feature/amazing-feature`
3. Make your changes
4. Run quality checks: `just ci`
5. Commit your changes: `git commit -m "feat: add amazing feature"`
6. Push to your fork: `git push origin feature/amazing-feature`
7. Open a Pull Request

## License

MIT License - see [LICENSE](LICENSE) file for details.

## Acknowledgments

Built with excellent Rust crates including `clap`, `rayon`, `dashmap`, `zstd`, and `xxhash-rust`.
