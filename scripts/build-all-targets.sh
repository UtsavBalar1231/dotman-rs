#!/usr/bin/env bash
# Simple script to build dotman for all supported targets
# Wrapper around cross-compile.sh with sensible defaults

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CROSS_COMPILE_SCRIPT="$SCRIPT_DIR/cross-compile.sh"

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${BLUE}[INFO]${NC} $1" >&2
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1" >&2
}

# Check if cross-compile script exists
if [[ ! -x "$CROSS_COMPILE_SCRIPT" ]]; then
    echo "Error: cross-compile.sh not found or not executable at $CROSS_COMPILE_SCRIPT"
    exit 1
fi

# Show what we're doing
log_info "Building dotman for all supported platforms..."
log_info "This will build optimized binaries for:"
log_info "  - Linux (x86_64 GNU, x86_64 musl, ARM64 GNU, ARM64 musl)"
log_info "  - macOS (Intel x86_64, Apple Silicon ARM64)"
log_info "  - Windows (x86_64)"

# Run the cross-compilation with distribution archives
"$CROSS_COMPILE_SCRIPT" --dist all "$@"

log_success "All target builds completed!"
log_success "Binaries available in target/{target}/release/"
log_success "Distribution archives available in dist/"
