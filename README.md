# dotman

[![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Build Status](https://img.shields.io/github/actions/workflow/status/UtsavBalar1231/dotman/ci.yml?branch=main)](https://github.com/UtsavBalar1231/dotman/actions)

High-performance dotfiles manager with Git-like semantics, built in Rust.

## Why dotman?

Dotman brings the power of Git to dotfiles management with a focus on **performance** and **simplicity**.

- **Git-like workflow** - Familiar commands: `add`, `commit`, `push`, `pull`, `branch`
- **Content deduplication** - Identical files stored once via xxHash3 (~30GB/s)
- **Blazing fast** - SIMD acceleration, parallel processing, memory-mapped I/O
- **Branch-based configs** - Different setups for different machines
- **Git remote support** - Backup to GitHub/GitLab/Bitbucket

## Installation

### From Source
```bash
git clone https://github.com/UtsavBalar1231/dotman.git
cd dotman
cargo install --path .
```

### Installation Script
```bash
curl -fsSL https://raw.githubusercontent.com/UtsavBalar1231/dotman/main/install/install.sh | bash
```

### Pre-built Binaries

Download from [releases](https://github.com/UtsavBalar1231/dotman/releases):

| Platform | Binary |
|----------|--------|
| Linux x86_64 (glibc) | `dotman-v{version}-x86_64-unknown-linux-gnu.tar.gz` |
| Linux x86_64 (musl) | `dotman-v{version}-x86_64-unknown-linux-musl.tar.gz` |
| Linux aarch64 | `dotman-v{version}-aarch64-unknown-linux-gnu.tar.gz` |

## Quick Start

```bash
# Initialize repository
dot init

# Track dotfiles
dot add ~/.zshrc ~/.vimrc ~/.config/nvim
dot commit -m "Initial dotfiles"

# Create machine-specific branch
dot checkout -b laptop
dot add ~/.config/i3
dot commit -m "Add laptop configs"

# Set up remote backup
dot remote add origin git@github.com:username/dotfiles.git
dot push -u origin main

# Restore on new machine
dot init
dot remote add origin git@github.com:username/dotfiles.git
dot pull origin main
```

## Commands

| Category | Commands |
|----------|----------|
| **Getting Started** | `init`, `add`, `commit`, `status` |
| **History** | `log`, `diff`, `show` |
| **Branching** | `branch`, `checkout`, `merge`, `rebase` |
| **Recovery** | `reset`, `restore`, `stash`, `revert` |
| **Remote** | `remote`, `push`, `pull`, `fetch` |
| **Utility** | `clean`, `config`, `tag`, `reflog`, `fsck` |

**Global flags:** `--verbose`, `--quiet`, `--no-pager`

Run `dot <command> --help` or `man dot-<command>` for detailed usage.

## Configuration

Config file: `~/.config/dotman/config` (TOML)

```toml
[user]
name = "Your Name"
email = "you@example.com"

[core]
compression_level = 3  # 1-22 (default: 3)

[tracking]
ignore_patterns = [".git", "*.swp", "*.tmp", "node_modules"]

[security]
# Path validation (default: enforce with $HOME only)
allowed_directories = ["/home/user"]
enforce_path_validation = true

# Permission sanitization (default: strip dangerous bits)
strip_dangerous_permissions = true
max_file_mode = 0o777
```

**Environment variables:**
- `DOTMAN_CONFIG_PATH` - Override config location
- `DOTMAN_REPO_PATH` - Override repository location (default: `~/.dotman`)

## Security

Dotman includes built-in security features to prevent common attacks:

### Path Traversal Protection

Prevents tracking files outside allowed directories (default: `$HOME` only).

```toml
[security]
# Allowed directories for tracking files
allowed_directories = ["/home/user", "/opt/custom"]

# Enforce path validation (default: true)
enforce_path_validation = true
```

**What's blocked:**
- `../../../etc/passwd` - Parent directory traversal
- `/etc/shadow` - Absolute paths outside allowed directories
- `~/../../../root` - Tilde bypass patterns
- Symlinks pointing outside allowed directories

**Error message example:**
```
Error: Path '/etc/passwd' is outside allowed directories.
Allowed directories:
  - /home/user

To track files in additional directories, edit ~/.config/dotman/config
```

### Permission Sanitization

Strips dangerous permission bits (setuid/setgid/sticky) from tracked files by default.

```toml
[security]
# Strip dangerous permission bits when storing (default: true)
strip_dangerous_permissions = true

# Maximum allowed file mode (default: 0o777)
max_file_mode = 0o777
```

**What's stripped:**
- `setuid (0o4000)` - Set user ID on execution
- `setgid (0o2000)` - Set group ID on execution
- `sticky (0o1000)` - Sticky bit

**Warning displayed when dangerous bits are stripped:**
```
Stripped dangerous permission bits from ~/.local/bin/script
  Original: 0o4755
  Sanitized: 0o755
  Removed: setuid (0o4000)

This is a security feature. Dangerous bits are stripped by default.
To disable: Set strip_dangerous_permissions = false in ~/.config/dotman/config (NOT RECOMMENDED)
```

**Security guarantees:**
- Dangerous bits are ALWAYS stripped on restore (even if disabled for storage)
- Normal permissions (rwxrwxrwx) are fully preserved
- Configuration allows opt-out for edge cases (with warnings)

### Why These Defaults?

**Path validation:** Prevents privilege escalation via unauthorized file access. Tracking `/etc/sudoers` or `/root/.ssh/authorized_keys` could allow attackers to gain root access when dotfiles are restored on a compromised system.

**Permission stripping:** Prevents setuid/setgid exploits. A malicious dotfile with setuid bit could run as root, enabling privilege escalation attacks.

### Migrating to 2.0.0

See `SECURITY_MIGRATION.md` for upgrading from older versions.

## Repository Structure

```
~/.dotman/
├── index.bin           # Staging area (staged files only)
├── tracking.bin        # Tracked directories/files manifest
├── commits/            # Zstd-compressed snapshots
├── objects/            # Content-addressed file storage
├── refs/
│   ├── heads/          # Local branches
│   ├── tags/           # Tags
│   └── remotes/        # Remote tracking refs (origin/*)
├── logs/HEAD           # Reflog for recovery
├── remote-mappings.toml # Git ↔ dotman commit mappings
├── MERGE_HEAD          # (merge in progress)
├── REBASE_STATE        # (rebase in progress)
└── HEAD                # Current branch pointer
```

## Architecture

```
┌─────────────────────────────────────┐
│           CLI (clap)                │
├─────────────────────────────────────┤
│     Staging-Only Index (v2)         │  Git-style two-stage model
│     • Staged files in index.bin     │
│     • Committed files in snapshots  │
├─────────────────────────────────────┤
│     Content Store (xxHash3)         │  Deduplication
│     Snapshots (Zstd compressed)     │  Full file storage
├─────────────────────────────────────┤
│     Performance Optimizations       │
│     • SIMD acceleration             │
│     • Parallel processing (rayon)   │
│     • Memory-mapped I/O (>1MB)      │
│     • Hash caching (size + mtime)   │
└─────────────────────────────────────┘
```

## Development

```bash
# Build
cargo build --release

# Test
cargo test

# Lint
cargo fmt && cargo clippy

# Run
./target/release/dot <command>

# Debug logging
RUST_LOG=debug ./target/release/dot status
```

Using [Just](https://github.com/casey/just) (recommended):
```bash
just build-release    # Release build
just test             # All tests
just lint             # Format + clippy
just ci               # Full CI suite
```

## Contributing

1. Fork and create a feature branch
2. Make changes with tests
3. Run `just ci` (or `cargo fmt && cargo clippy && cargo test`)
4. Open a Pull Request

See [SECURITY.md](SECURITY.md) for vulnerability reporting.

## License

MIT License - see [LICENSE](LICENSE) for details.
