# dotman-rs

`dotman-rs` is a fast and lightweight dotfiles management system written in Rust. It provides a seamless command-line interface (CLI) to track, version, and synchronize your dotfiles across multiple systems, ensuring your configurations are always up-to-date.

## Features

- **Track Configurations**: Manage mappings between your system files and a central dotfiles repository.
- **Push and Pull**: Sync configurations between your system and the dotfiles repository with intelligent change detection.
- **Force Updates**: Overwrite configurations in either direction when needed.
- **Efficient Hashing**: Uses `blake3` for fast and secure file hashing with caching for optimal performance.
- **Parallel Operations**: Leverages `rayon` for handling large file sets efficiently.
- **Metadata Management**: Clear, fix, or edit cached metadata to keep your setup clean and consistent.

## Installation

1. Install Rust using [rustup](https://rustup.rs/):

   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

1. Clone the repository:

   ```bash
   git clone https://github.com/UtsavBalar1231/dotman-rs.git
   cd dotman-rs
   ```

1. Build and install:

   ```bash
   cargo install --path .
   ```

## Usage

Run `dotman-rs --help` to see all available commands:

```text
Easily Manage dotfiles across machines

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
  -c, --config-path <CONFIG_PATH>  Provide custom path to the config file (default: ${pwd}/config.toml)
  -h, --help                       Print help
  -V, --version                    Print version
```

### Examples

#### Add a Configuration

Track a file or directory:

```bash
dotman-rs add -n my-config -p ~/.config/my-app/config
```

#### Push Configurations

Sync tracked configurations to the dotfiles repository:

```bash
dotman-rs local-push
```

#### Pull Configurations

Restore tracked configurations from the dotfiles repository:

```bash
dotman-rs local-pull
```

#### Clear Metadata

Remove all cached metadata:

```bash
dotman-rs clear-metadata
```

#### Edit Configuration

Open the configuration file in your default editor:

```bash
dotman-rs edit
```

#### Fix Metadata

Resolve stale or corrupted metadata entries:

```bash
dotman-rs fix-config
```

### Configuration File

`dotman-rs` uses a TOML file to manage configuration mappings and metadata. The configuration file tracks system paths, their corresponding hashes, and the type of configuration (file or directory).  

The default configuration file is located at `~/.config/dotman-rs/config.toml`. You can customize its location using the `DOTMAN_CONFIG_PATH` environment variable.  

#### Example Configuration File  

```toml
[dotconfigs_path]
Local = "$HOME/dotfiles/configs/"

[[configs]]
name = "nvim"
path = "$HOME/.config/nvim/"
hash = "b91052c7e9fe1b05f51c9597b7fe28f38204ef9f1e8b11d79c7194dda2b28170"
conf_type = "Dir"

[[configs]]
name = "polybar"
path = "$HOME/.config/polybar/"
hash = "566356a8489ae372b717771126952f72b71eca6dc5aa0002d6b40f6dea685b9a"
conf_type = "Dir"

[[configs]]
name = "rofi"
path = "$HOME/.config/rofi/"
hash = "d50adfc53405f4f0f5904c3751f00b4963977e34c42e1525be05bec8b8a81135"
conf_type = "Dir"
```

#### Fields Description  

- **`[dotconfigs_path]`**  
  - Specifies the local directory where dotfiles are tracked.  

- **`[[configs]]`**  
  - **`name`**: A unique name for the configuration.  
  - **`path`**: The absolute or environment-variable-based path to the system configuration.  
  - **`hash`**: The cached hash of the configuration for detecting changes.  
  - **`conf_type`**: The type of configuration (`File` or `Dir`).  

This structure ensures efficient tracking and synchronization of your dotfiles.

## Contributing

1. Fork the repository.
2. Create a new branch for your feature or bugfix.
3. Commit your changes and push to your fork.
4. Submit a pull request.

## License

This project is licensed under the [MIT License](LICENSE).

---

## TODO

1. Add support for tracking remote repositories.
   - Implement `dotman-rs remote-push` and `dotman-rs remote-pull`.
2. Add support for checking status of tracked configurations.
   - Implement `dotman-rs status` for displaying the status of tracked configurations.
3. Use `fs_extra` for efficient file operations.
4. Do Documentation.
