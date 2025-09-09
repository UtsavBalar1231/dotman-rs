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

### Pre-built Binaries
Download from [releases](https://github.com/yourusername/dotman/releases) for your platform:
- Linux (x86_64, ARM64)
- macOS (Intel, Apple Silicon)
- Windows (x86_64)

### Package Managers
```bash
# Arch Linux (AUR)
yay -S dotman

# Homebrew (macOS/Linux)
brew install dotman

# Cargo
cargo install dotman
```

## Quick Start

```bash
# Initialize repository
dot init

# Track dotfiles
dot add ~/.zshrc ~/.vimrc ~/.config/nvim
dot commit -m "Initial dotfiles"

# Create machine-specific branch
dot branch laptop
dot checkout laptop
dot add ~/.config/i3
dot commit -m "Add laptop-specific configs"

# Set up remote backup
dot remote add origin git@github.com:yourusername/dotfiles.git
dot push origin main

# Restore on new machine
dot init
dot remote add origin git@github.com:yourusername/dotfiles.git
dot pull origin main
dot checkout
```

## Core Commands

### Essential Operations
| Command | Description |
|---------|------------|
| `dot init` | Initialize a new repository |
| `dot add <files>` | Track files or directories |
| `dot status` | Show tracked/modified files |
| `dot commit -m "msg"` | Create a snapshot |
| `dot checkout [ref]` | Restore files from snapshot |
| `dot reset <ref>` | Reset to a specific commit |

### Branching & History
| Command | Description |
|---------|------------|
| `dot branch [name]` | Create or list branches |
| `dot checkout -b <name>` | Create and switch branch |
| `dot log` | View commit history |
| `dot reflog` | View HEAD movement history |
| `dot tag <name>` | Create named reference |

### Remote Operations
| Command | Description |
|---------|------------|
| `dot remote add <name> <url>` | Add remote repository |
| `dot push [remote] [branch]` | Push changes |
| `dot pull [remote] [branch]` | Pull and merge changes |
| `dot fetch [remote]` | Download remote changes |

### Maintenance
| Command | Description |
|---------|------------|
| `dot clean` | Remove untracked files |
| `dot gc` | Garbage collect unreferenced objects |
| `dot config <key> <value>` | Set configuration |

## Configuration

Dotman uses TOML configuration at `~/.config/dotman/config`:

```toml
[core]
compression_level = 3        # 1-22, higher = better compression
pager = "less"              # Optional pager for output

[user]
name = "Your Name"
email = "you@example.com"

[performance]
parallel_threads = 0         # 0 = auto-detect
mmap_threshold = 1048576    # Use mmap for files > 1MB

[tracking]
ignore_patterns = [".git", "*.swp", "node_modules"]
follow_symlinks = false
preserve_permissions = true
```

## Architecture

Dotman uses a **trait-based architecture** with content-addressed storage for maximum performance and code reuse:

```
┌─────────────────────────────────────────┐
│            CLI (clap)                   │
├─────────────────────────────────────────┤
│     CommandContext Trait                │  ← Shared command logic
│     RemoteOperations Trait              │  ← Remote sync operations
├─────────────────────────────────────────┤
│     Binary Index (DashMap)              │  ← O(1) lookups
│     Content Store (xxHash3)             │  ← Deduplication
│     Snapshots (Zstd compressed)         │  ← Commit storage
├─────────────────────────────────────────┤
│     SIMD UTF-8 & JSON                   │  ← Hardware acceleration
│     Memory-mapped I/O                   │  ← Large file handling
│     Parallel Processing (rayon)         │  ← Multi-core utilization
└─────────────────────────────────────────┘
```

### Repository Structure
```
~/.dotman/
├── index.bin          # Binary index
├── commits/           # Snapshot storage
├── objects/           # Deduplicated content
├── refs/              # Branches and tags
├── logs/HEAD          # Reflog
└── HEAD              # Current reference
```

## Development

```bash
# Build
cargo build --release

# Run tests
cargo test

# Run with verbose output
RUST_LOG=debug dot status

# Benchmarks
cargo bench

# Code quality
cargo fmt
cargo clippy
```

### Using Just (Task Runner)
```bash
just build            # Debug build
just test            # Run all tests
just ci              # Full CI suite
just cross-compile all  # Build for all platforms
```

## Contributing

Contributions welcome! Please ensure:
- Tests pass: `cargo test`
- Code is formatted: `cargo fmt`
- Clippy is happy: `cargo clippy`

See [CONTRIBUTING.md](CONTRIBUTING.md) for detailed guidelines.

## Performance

Dotman is optimized for speed at every level:
- **SIMD acceleration** for UTF-8 validation and JSON parsing
- **Binary index** with lock-free concurrent access via DashMap
- **Memory-mapped I/O** for large files (>1MB)
- **Parallel operations** via rayon with configurable thread count
- **xxHash3** for content hashing (>30GB/s on modern CPUs)
- **Zstandard compression** with dictionary training

## License

MIT License - see [LICENSE](LICENSE) file for details.

## Acknowledgments

Built with excellent Rust crates including `clap`, `rayon`, `dashmap`, `zstd`, and `xxhash-rust`.