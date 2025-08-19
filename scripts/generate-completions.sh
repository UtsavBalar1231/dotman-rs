#!/usr/bin/env bash
# Generate shell completion scripts for dotman
# This script builds the binary and generates completion scripts for all supported shells

set -euo pipefail

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${BLUE}[INFO]${NC} $1" >&2
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1" >&2
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1" >&2
}

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
COMPLETIONS_DIR="$PROJECT_ROOT/completions"
BINARY_PATH="$PROJECT_ROOT/target/release/dot"

# Create completions directory
mkdir -p "$COMPLETIONS_DIR"

log_info "Building dotman binary..."

# Build the binary first
cd "$PROJECT_ROOT"
if ! cargo build --release --bin dot; then
    echo "Error: Failed to build dotman binary" >&2
    exit 1
fi

# Verify binary exists
if [[ ! -f "$BINARY_PATH" ]]; then
    echo "Error: Binary not found at $BINARY_PATH" >&2
    exit 1
fi

log_success "Binary built successfully"

# Generate completions for each shell
log_info "Generating shell completions..."

# Bash completion
log_info "Generating Bash completion..."
"$BINARY_PATH" completion bash > "$COMPLETIONS_DIR/dot.bash"
log_success "Generated Bash completion: completions/dot.bash"

# Zsh completion
log_info "Generating Zsh completion..."
"$BINARY_PATH" completion zsh > "$COMPLETIONS_DIR/_dot"
log_success "Generated Zsh completion: completions/_dot"

# Fish completion  
log_info "Generating Fish completion..."
"$BINARY_PATH" completion fish > "$COMPLETIONS_DIR/dot.fish"
log_success "Generated Fish completion: completions/dot.fish"

# PowerShell completion (for completeness)
log_info "Generating PowerShell completion..."
"$BINARY_PATH" completion powershell > "$COMPLETIONS_DIR/dot.ps1"
log_success "Generated PowerShell completion: completions/dot.ps1"

# Elvish completion
log_info "Generating Elvish completion..."
"$BINARY_PATH" completion elvish > "$COMPLETIONS_DIR/dot.elv"
log_success "Generated Elvish completion: completions/dot.elv"

log_success "All shell completions generated successfully!"

# Show installation instructions
cat << EOF

INSTALLATION INSTRUCTIONS:

Bash:
  Copy completions/dot.bash to one of:
  - /usr/share/bash-completion/completions/dot
  - /usr/local/share/bash-completion/completions/dot
  - ~/.local/share/bash-completion/completions/dot
  
  Or source it directly in ~/.bashrc:
    source /path/to/completions/dot.bash

Zsh:
  Copy completions/_dot to a directory in your \$fpath:
  - /usr/share/zsh/site-functions/_dot
  - ~/.local/share/zsh/site-functions/_dot
  
  Or add to ~/.zshrc:
    fpath=(~/.local/share/zsh/site-functions \$fpath)
    autoload -Uz compinit && compinit

Fish:
  Copy completions/dot.fish to:
  - ~/.config/fish/completions/dot.fish
  - /usr/share/fish/vendor_completions.d/dot.fish

PowerShell:
  Add to your PowerShell profile:
    . /path/to/completions/dot.ps1

Elvish:
  Add to ~/.elvish/rc.elv:
    eval (cat /path/to/completions/dot.elv)

EOF