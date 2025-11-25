#!/usr/bin/env bash
# Install shell completion scripts for dotman
# This script installs either basic or enhanced completions

set -euo pipefail

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
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

log_error() {
    echo -e "${RED}[ERROR]${NC} $1" >&2
}

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
COMPLETIONS_DIR="$PROJECT_ROOT/completions"

# Parse arguments
SHELL_TYPE="${1:-}"
COMPLETION_TYPE="${2:-enhanced}"  # Default to enhanced

usage() {
    cat << EOF
Usage: $0 <shell> [basic|enhanced]

Install dotman shell completions for the specified shell.

Arguments:
  shell      The shell to install completions for (bash, zsh, fish)
  type       The type of completions to install (basic or enhanced)
             Default: enhanced

Examples:
  $0 bash               # Install enhanced Bash completions
  $0 zsh basic         # Install basic Zsh completions
  $0 fish enhanced     # Install enhanced Fish completions

Supported shells:
  - bash: Installs to ~/.local/share/bash-completion/completions/
  - zsh:  Installs to ~/.local/share/zsh/site-functions/
  - fish: Installs to ~/.config/fish/completions/

EOF
    exit 1
}

if [[ -z "$SHELL_TYPE" ]]; then
    usage
fi

if [[ "$COMPLETION_TYPE" != "basic" && "$COMPLETION_TYPE" != "enhanced" ]]; then
    log_error "Invalid completion type: $COMPLETION_TYPE"
    usage
fi

# Detect OS
OS="$(uname -s)"

install_bash_completions() {
    local source_file
    if [[ "$COMPLETION_TYPE" == "enhanced" ]]; then
        source_file="$COMPLETIONS_DIR/dot-enhanced.bash"
    else
        source_file="$COMPLETIONS_DIR/dot.bash"
    fi

    if [[ ! -f "$source_file" ]]; then
        log_error "Completion file not found: $source_file"
        log_info "Run './scripts/generate-completions.sh' first to generate completions"
        exit 1
    fi

    # Determine installation directory
    local install_dir
    if [[ "$OS" == "Darwin" ]]; then
        # macOS with Homebrew
        if command -v brew &> /dev/null; then
            install_dir="$(brew --prefix)/share/bash-completion/completions"
        else
            install_dir="$HOME/.local/share/bash-completion/completions"
        fi
    else
        # Linux
        install_dir="$HOME/.local/share/bash-completion/completions"
    fi

    # Create directory if it doesn't exist
    mkdir -p "$install_dir"

    # Install completion
    cp "$source_file" "$install_dir/dot"

    log_success "Installed $COMPLETION_TYPE Bash completions to $install_dir/dot"

    # Check if bash-completion is loaded in .bashrc
    if ! grep -q "bash-completion" "$HOME/.bashrc" 2>/dev/null; then
        log_warning "bash-completion might not be loaded in your .bashrc"
        log_info "Add the following to your ~/.bashrc to enable completions:"
        echo ""
        echo "  # Load bash completions"
        echo "  if [ -f /etc/bash_completion ]; then"
        echo "      . /etc/bash_completion"
        echo "  fi"
        echo ""
        echo "  # Load user completions"
        echo "  if [ -d ~/.local/share/bash-completion/completions ]; then"
        echo "      for file in ~/.local/share/bash-completion/completions/*; do"
        echo "          [ -r \"\$file\" ] && . \"\$file\""
        echo "      done"
        echo "  fi"
    fi

    log_info "Reload your shell or run 'source ~/.bashrc' to activate completions"
}

install_zsh_completions() {
    local source_file
    if [[ "$COMPLETION_TYPE" == "enhanced" ]]; then
        source_file="$COMPLETIONS_DIR/_dot-enhanced"
    else
        source_file="$COMPLETIONS_DIR/_dot"
    fi

    if [[ ! -f "$source_file" ]]; then
        log_error "Completion file not found: $source_file"
        log_info "Run './scripts/generate-completions.sh' first to generate completions"
        exit 1
    fi

    # Determine installation directory
    local install_dir="$HOME/.local/share/zsh/site-functions"

    # Create directory if it doesn't exist
    mkdir -p "$install_dir"

    # Install completion
    cp "$source_file" "$install_dir/_dot"

    log_success "Installed $COMPLETION_TYPE Zsh completions to $install_dir/_dot"

    # Check if fpath includes our directory
    if ! grep -q "fpath.*\.local/share/zsh/site-functions" "$HOME/.zshrc" 2>/dev/null; then
        log_warning "The completion directory might not be in your fpath"
        log_info "Add the following to your ~/.zshrc before 'compinit':"
        echo ""
        echo "  # Add local completions to fpath"
        echo "  fpath=(~/.local/share/zsh/site-functions \$fpath)"
        echo "  autoload -Uz compinit && compinit"
    fi

    log_info "Run 'rm ~/.zcompdump && compinit' to rebuild completion cache"
}

install_fish_completions() {
    local source_file
    if [[ "$COMPLETION_TYPE" == "enhanced" ]]; then
        source_file="$COMPLETIONS_DIR/dot-enhanced.fish"
    else
        source_file="$COMPLETIONS_DIR/dot.fish"
    fi

    if [[ ! -f "$source_file" ]]; then
        log_error "Completion file not found: $source_file"
        log_info "Run './scripts/generate-completions.sh' first to generate completions"
        exit 1
    fi

    # Fish completions directory
    local install_dir="$HOME/.config/fish/completions"

    # Create directory if it doesn't exist
    mkdir -p "$install_dir"

    # Install completion
    cp "$source_file" "$install_dir/dot.fish"

    log_success "Installed $COMPLETION_TYPE Fish completions to $install_dir/dot.fish"
    log_info "Fish will automatically load the completions on next start"
}

# Main installation logic
case "$SHELL_TYPE" in
    bash)
        install_bash_completions
        ;;
    zsh)
        install_zsh_completions
        ;;
    fish)
        install_fish_completions
        ;;
    *)
        log_error "Unsupported shell: $SHELL_TYPE"
        log_info "Supported shells: bash, zsh, fish"
        exit 1
        ;;
esac
