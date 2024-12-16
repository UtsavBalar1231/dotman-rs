# dotman-rs

`dotman-rs` is a fast, light weight dotfiles management system written in Rust.
It provides an intuitive command-line interface (CLI) to easily manage, version, and synchronize your dotfiles across systems.

## Features

- **Add Configurations**: Track your system files and manage their mappings to your dotfiles repository.
- **Push Configurations**: Synchronize local configurations into your dotfiles repository.
- **Pull Configurations**: Restore tracked files from your dotfiles repository to your system.
- **Clear Metadata**: Remove all cached metadata and reset the state of `dotman-rs`.
- **Edit Configuration**: Open the configuration file in your favorite editor for manual tweaks.
- **Fix Metadata**: Resolve stale or corrupted entries in the metadata cache.

## Installation

1. Install Rust using [rustup](https://rustup.rs/):
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. Clone the repository:
   ```bash
   git clone https://github.com/UtsavBalar1231/dotman-rs.git
   cd dotman-rs
   ```

3. Build and install:
   ```bash
   cargo install --path .
   ```

## Usage

Run `dotman-rs --help` to see all available commands:

```bash
Usage: dotman-rs [OPTIONS] <COMMAND>

Commands:
  local-pull, -p      Push configs from tracking config directory into your local system
  local-push, -u      Pull configs from your local system into the tracking config directory
  force-pull, -P      Force pull your tracking config directory with the latest configs from your local system
  force-push, -U      Force Update your local system with the configs from the tracking directory
  clear-metadata, -x  Clear the metadata of config entries in the dotman config
  new, -n             Prints a new dotman configuration
  printconf, -r       Prints the currently used dotman config file
  fix-config, -z      Fix your dotman config for any errors
  add, -a             Adds a new config entry to your exisiting dotman config
  edit, -e            Edit the dotman config file in your editor
  clean, -C           Clean the dotconfigs directory
  help                Print this message or the help of the given subcommand(s)

Options:
  -c, --config-path <CONFIG_PATH>  Provide custom path to the config file (default: ${pwd}/config.ron)
  -h, --help                       Print help
  -V, --version                    Print version
```

### Examples

#### Add a Configuration
Track a system configuration file:
```bash
dotman-rs add -n my-config -p ~/.config/my-app/config

dotman-rs -a -n my-config -p ~/.config/my-app/config
```

#### Push Configurations
Push the tracked configurations to the local dotfiles repository:
```bash
dotman-rs -u
```

```bash
dotman-rs localpush
```

#### Pull Configurations
Restore all tracked configurations from the local dotfiles repository:
```bash
dotman-rs -p
```

```bash
dotman-rs localpull
```

#### Clear Metadata
Clear all metadata and reset the state:
```bash
dotman-rs clear
```

```bash
dotman-rs -x
```

#### Edit Configuration
Manually edit the configuration file:
```bash
dotman-rs edit
```

```bash
dotman-rs -e
```

#### Fix Metadata
Fix stale or corrupted metadata entries:
```bash
dotman-rs fix
```

```bash
dotman-rs -z
```

## Configuration File

`dotman-rs` uses a RON (Rusty Object Notation) file for storing configuration data. This file keeps mappings of your tracked files and cached hashes.

### Example Configuration File
```ron
(
    (
        name: "my-config",
        system_path: "~/.config/my-app/config",
        dotfile_path: "~/dotfiles/my-app-config",
    )
)
```

The default configuration file is located at `~/.config/dotman-rs/config.ron`. You can customize its path using the `DOTMAN_CONFIG_PATH` environment variable.

## Advanced Features

### Parallel Operations
`dotman-rs` leverages `rayon` for parallel operations, ensuring fast execution when managing large numbers of files.

### Metadata Caching
Efficient hashing and caching reduce redundant calculations and improve performance.

### Editor Integration
`dotman-rs` automatically opens your configuration file using the editor specified in the `EDITOR` environment variable.

## Contributing

1. Fork the repository.
2. Create a new branch for your feature or bugfix.
3. Commit your changes and push to your fork.
4. Submit a pull request.

## License

This project is licensed under the [MIT License](LICENSE).

---

Start managing your dotfiles effortlessly with `dotman-rs`! For questions or feedback, feel free to open an issue or contribute to the project.

## TODO

1. Add support for remote dotfiles repository (github)
2. rayon wherever possible
3. Documentation and Examples
4. Optimize File Copying using `fs_extra`
```rust
use fs_extra::dir::{copy as copy_dir, CopyOptions as DirCopyOptions};
use fs_extra::file::{copy as copy_file, CopyOptions as FileCopyOptions};

impl Config {
    fn copy_with_fs_extra(src: &Path, dst: &Path) -> io::Result<()> {
        if src.is_dir() {
            let mut options = DirCopyOptions::new();
            options.overwrite = true; // Overwrite existing files
            options.copy_inside = true; // Copy contents of the directory
            copy_dir(src, dst, &options).map_err(|err| {
                io::Error::new(io::ErrorKind::Other, format!("Failed to copy directory: {}", err))
            })?;
        } else if src.is_file() {
            let mut options = FileCopyOptions::new();
            options.overwrite = true; // Overwrite existing file
            copy_file(src, dst, &options).map_err(|err| {
                io::Error::new(io::ErrorKind::Other, format!("Failed to copy file: {}", err))
            })?;
        } else {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("Source path does not exist: {}", src.display()),
            ));
        }
        Ok(())
    }
}

pub fn pull_config(&mut self, clean: bool) -> io::Result<()> {
    let backup_path = &self.dotconfigs_path.get_path();

    if !backup_path.exists() {
        fs::create_dir_all(backup_path)?;
    }

    for entry in &mut self.configs {
        let src_path = entry.path.clone();
        let current_hash = if src_path.is_dir() {
            let mut hasher = Sha1::new();
            hasher::get_complete_dir_hash(&src_path, &mut hasher).unwrap_or_default()
        } else if src_path.is_file() {
            let mut hasher = Sha1::new();
            hasher::get_file_hash(&src_path, &mut hasher).unwrap_or_default()
        } else {
            String::new()
        };

        if entry.hash != current_hash || clean {
            println!("Pulling: {}", entry.name);
            let dest_path = backup_path.join(&entry.name);
            Self::copy_with_fs_extra(&src_path, &dest_path)?;
            entry.hash = current_hash;
        } else {
            println!("No changes detected for: {}", entry.name);
        }
    }

    self.save_config()
}

pub fn push_config(&self, clean: bool) -> Result<(), ConfigError> {
    let backup_path = &self.dotconfigs_path.get_path();

    if !backup_path.exists() {
        fs::create_dir_all(backup_path)?;
    }

    for entry in &self.configs {
        let src_path = backup_path.join(&entry.name);
        let dst_path = entry.path.clone();

        if clean {
            if dst_path.exists() {
                if dst_path.is_dir() {
                    fs::remove_dir_all(&dst_path)?;
                } else {
                    fs::remove_file(&dst_path)?;
                }
            }
        }

        println!("Pushing: {}", entry.name);
        Self::copy_with_fs_extra(&src_path, &dst_path)?;
    }

    self.save_config()
}

```

