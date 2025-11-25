#!/usr/bin/env bash
# Cross-compilation script for dotman
# Builds optimized binaries for all target platforms

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
BUILD_DIR="$PROJECT_ROOT/target"
DIST_DIR="$PROJECT_ROOT/dist"

# Project info
PROJECT_NAME="dotman"
BINARY_NAME="dot"
VERSION=$(grep '^version = ' "$PROJECT_ROOT/Cargo.toml" | sed 's/.*"\(.*\)".*/\1/')

# Default settings
OPTIMIZE=true
STRIP=true
PARALLEL_JOBS=$(nproc 2>/dev/null || sysctl -n hw.ncpu 2>/dev/null || echo "4")
VERBOSE=false
TARGET=""

# Logging functions
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

# Help function
show_help() {
	cat <<EOF
Cross-compilation script for $PROJECT_NAME v$VERSION

USAGE:
    $0 [OPTIONS] [TARGET]

TARGETS:
    all                    Build for all supported platforms
    linux                  Build for all Linux targets
    linux-x86_64           Build for Linux x86_64 GNU
    linux-aarch64         Build for Linux ARM64 GNU
    macos                  Build for all macOS targets
    macos-x86_64          Build for macOS Intel
    macos-aarch64         Build for macOS Apple Silicon
    windows                Build for all Windows targets
    windows-x86_64        Build for Windows x86_64

OPTIONS:
    -h, --help            Show this help message
    -v, --verbose         Enable verbose output
    -j, --jobs N          Number of parallel jobs (default: $PARALLEL_JOBS)
    --no-optimize         Disable optimizations
    --no-strip            Don't strip debug symbols
    --dist                Create distribution archives
    --clean               Clean build artifacts before building

EXAMPLES:
    $0 all                # Build for all platforms
    $0 linux              # Build for all Linux targets
    $0 macos-aarch64      # Build for Apple Silicon only
    $0 --dist all         # Build all and create distribution archives
    $0 -j 8 linux         # Use 8 parallel jobs for Linux builds

ENVIRONMENT VARIABLES:
    CARGO                 Cargo command (default: cargo)
    RUSTFLAGS             Additional Rust compiler flags
    CROSS                 Use cross for compilation (default: auto-detect)
EOF
}

# Parse command line arguments
parse_args() {
	local create_dist=false
	local clean_first=false

	while [[ $# -gt 0 ]]; do
		case $1 in
		-h | --help)
			show_help
			exit 0
			;;
		-v | --verbose)
			VERBOSE=true
			shift
			;;
		-j | --jobs)
			PARALLEL_JOBS="$2"
			shift 2
			;;
		--no-optimize)
			OPTIMIZE=false
			shift
			;;
		--no-strip)
			STRIP=false
			shift
			;;
		--dist)
			create_dist=true
			shift
			;;
		--clean)
			clean_first=true
			shift
			;;
		-*)
			log_error "Unknown option: $1"
			exit 1
			;;
		*)
			if [[ -z "$TARGET" ]]; then
				TARGET="$1"
			else
				log_error "Multiple targets specified: $TARGET and $1"
				exit 1
			fi
			shift
			;;
		esac
	done

	# Default to 'all' if no target specified
	if [[ -z "$TARGET" ]]; then
		TARGET="all"
	fi

	# Clean if requested
	if [[ "$clean_first" == "true" ]]; then
		log_info "Cleaning build artifacts..."
		cargo clean
	fi

	# Create dist directory if needed
	if [[ "$create_dist" == "true" ]]; then
		mkdir -p "$DIST_DIR"
	fi

	export CREATE_DIST="$create_dist"
}

# Check dependencies
check_dependencies() {
	local missing_deps=()

	# Check for required tools
	if ! command -v cargo >/dev/null; then
		missing_deps+=("cargo")
	fi

	# Always use cargo (cross tool removed)
	export CARGO_CMD="cargo"
	log_info "Using native cargo for compilation"

	if [[ ${#missing_deps[@]} -gt 0 ]]; then
		log_error "Missing required dependencies: ${missing_deps[*]}"
		log_error "Please install missing dependencies and try again"
		exit 1
	fi

	# Check if target toolchains are installed for cross-compilation
	check_target_toolchains
}

# Check if required target toolchains are installed
check_target_toolchains() {
	local target="$TARGET"

	# Only check for non-native targets
	if [[ "$target" == "all" ]] || [[ "$target" == "linux" ]] || [[ "$target" == *"aarch64"* ]]; then
		# Check if aarch64 target is installed
		if ! rustup target list --installed | grep -q "aarch64-unknown-linux-gnu"; then
			log_warning "Target 'aarch64-unknown-linux-gnu' not installed"
			log_info "Installing target with: rustup target add aarch64-unknown-linux-gnu"
			rustup target add aarch64-unknown-linux-gnu || {
				log_error "Failed to install aarch64-unknown-linux-gnu target"
				log_error "Please install it manually with: rustup target add aarch64-unknown-linux-gnu"
				exit 1
			}
		fi

		# Check for aarch64 linker
		if ! command -v aarch64-linux-gnu-gcc >/dev/null 2>&1; then
			log_warning "Cross-compilation linker 'aarch64-linux-gnu-gcc' not found"
			log_info "On Ubuntu/Debian: sudo apt-get install gcc-aarch64-linux-gnu"
			log_info "On Fedora: sudo dnf install gcc-aarch64-linux-gnu"
			log_info "On Arch: sudo pacman -S aarch64-linux-gnu-gcc"
		fi
	fi
}

# Build configuration
get_rustflags() {
	local target="$1"
	local base_flags="-C opt-level=3"

	if [[ "$OPTIMIZE" == "true" ]]; then
		base_flags+=" -C lto=fat -C codegen-units=1"

		# Target-specific optimizations
		case "$target" in
		aarch64-*)
			base_flags+=" -C target-cpu=generic"
			;;
		x86_64-*)
			base_flags+=" -C target-cpu=x86-64-v2"
			;;
		esac
	fi

	if [[ "$STRIP" == "true" ]]; then
		base_flags+=" -C strip=symbols"
	fi

	# Add user-provided RUSTFLAGS
	if [[ -n "${RUSTFLAGS:-}" ]]; then
		base_flags+=" $RUSTFLAGS"
	fi

	echo "$base_flags"
}

# Build function
build_target() {
	local target="$1"
	local target_name

	case "$target" in
	x86_64-unknown-linux-gnu)
		target_name="Linux x86_64 (GNU)"
		;;
	aarch64-unknown-linux-gnu)
		target_name="Linux ARM64 (GNU)"
		;;
	# 	;;
	x86_64-apple-darwin)
		target_name="macOS Intel"
		;;
	aarch64-apple-darwin)
		target_name="macOS Apple Silicon"
		;;
	x86_64-pc-windows-gnu)
		target_name="Windows x86_64"
		;;
	*)
		target_name="$target"
		;;
	esac

	log_info "Building $PROJECT_NAME for $target_name..."

	local rustflags
	rustflags=$(get_rustflags "$target")

	local cargo_args=(
		"build"
		"--release"
		"--target" "$target"
		"--all-features"
	)

	if [[ "$VERBOSE" == "true" ]]; then
		cargo_args+=("--verbose")
	fi

	# Set cross-compilation linkers based on target
	local env_vars=""
	case "$target" in
	aarch64-unknown-linux-gnu)
		env_vars="CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc"
		;;
	x86_64-pc-windows-gnu)
		env_vars="CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=x86_64-w64-mingw32-gcc"
		;;
	# 	;;
	esac

	# Set environment and build
	if ! env $env_vars RUSTFLAGS="$rustflags" \
		"$CARGO_CMD" "${cargo_args[@]}"; then
		log_error "Failed to build for $target"
		log_error "Make sure you have the target installed: rustup target add $target"
		if [[ "$target" == "aarch64-unknown-linux-gnu" ]]; then
			log_error "Also ensure you have the cross-compilation toolchain: sudo apt-get install gcc-aarch64-linux-gnu"
		fi
		exit 1
	fi

	# Verify binary was created
	local binary_path="$BUILD_DIR/$target/release/$BINARY_NAME"
	if [[ "$target" == *"windows"* ]]; then
		binary_path+=".exe"
	fi

	if [[ ! -f "$binary_path" ]]; then
		log_error "Binary not found at $binary_path"
		exit 1
	fi

	# Show binary info
	local size
	size=$(stat -f%z "$binary_path" 2>/dev/null || stat -c%s "$binary_path" 2>/dev/null || echo "unknown")
	log_success "Built $target_name: $binary_path ($(numfmt --to=iec "$size" 2>/dev/null || echo "$size bytes"))"

	# Create distribution archive if requested
	if [[ "${CREATE_DIST:-false}" == "true" ]]; then
		create_distribution_archive "$target" "$binary_path"
	fi
}

# Create distribution archive
create_distribution_archive() {
	local target="$1"
	local binary_path="$2"

	# Convert target triple to simplified naming
	local simplified_name=""
	case "$target" in
	x86_64-unknown-linux-gnu)
		simplified_name="$PROJECT_NAME-v$VERSION-x86_64-linux-gnu"
		;;
	aarch64-unknown-linux-gnu)
		simplified_name="$PROJECT_NAME-v$VERSION-aarch64-linux-gnu"
		;;
	x86_64-unknown-linux-musl)
		simplified_name="$PROJECT_NAME-v$VERSION-x86_64-linux-musl"
		;;
	aarch64-unknown-linux-musl)
		simplified_name="$PROJECT_NAME-v$VERSION-aarch64-linux-musl"
		;;
	x86_64-apple-darwin)
		simplified_name="$PROJECT_NAME-v$VERSION-x86_64-darwin"
		;;
	aarch64-apple-darwin)
		simplified_name="$PROJECT_NAME-v$VERSION-aarch64-darwin"
		;;
	x86_64-pc-windows-gnu)
		simplified_name="$PROJECT_NAME-v$VERSION-x86_64-windows"
		;;
	*)
		simplified_name="$PROJECT_NAME-v$VERSION-$target"
		;;
	esac

	local archive_name="$simplified_name"

	log_info "Creating distribution archive for $target..."

	# Save current directory
	local orig_dir=$(pwd)

	# Create temporary directory for archive contents
	local temp_dir
	temp_dir=$(mktemp -d)
	local archive_dir="$temp_dir/$archive_name"
	mkdir -p "$archive_dir"

	# Copy binary
	cp "$binary_path" "$archive_dir/"

	# Copy additional files
	if [[ -f "$PROJECT_ROOT/README.md" ]]; then
		cp "$PROJECT_ROOT/README.md" "$archive_dir/"
	fi
	if [[ -f "$PROJECT_ROOT/LICENSE" ]]; then
		cp "$PROJECT_ROOT/LICENSE" "$archive_dir/"
	fi
	if [[ -f "$PROJECT_ROOT/CHANGELOG.md" ]]; then
		cp "$PROJECT_ROOT/CHANGELOG.md" "$archive_dir/"
	fi

	# Create archive
	cd "$temp_dir"
	if [[ "$target" == *"windows"* ]]; then
		# Use zip for Windows
		if command -v zip >/dev/null; then
			zip -r "$DIST_DIR/$archive_name.zip" "$archive_name" >/dev/null
			log_success "Created $archive_name.zip"
		else
			log_warning "zip not found, skipping Windows archive creation"
		fi
	else
		# Use tar.gz for Unix-like systems
		tar czf "$DIST_DIR/$archive_name.tar.gz" "$archive_name"
		log_success "Created $archive_name.tar.gz"
	fi

	# Cleanup
	rm -rf "$temp_dir"

	# Return to original directory
	cd "$orig_dir"
}

# Main build function
build_targets() {
	local target="$1"

	case "$target" in
	all)
		build_targets linux
		# Note: macOS and Windows targets are currently disabled
		# Uncomment the following lines when cross-compilation toolchains are available:
		# build_targets macos
		# build_targets windows
		;;
	linux)
		build_target "x86_64-unknown-linux-gnu"
		build_target "aarch64-unknown-linux-gnu"
		;;
	linux-x86_64)
		build_target "x86_64-unknown-linux-gnu"
		;;
	linux-aarch64)
		build_target "aarch64-unknown-linux-gnu"
		;;
	# macos)
	# 	build_target "x86_64-apple-darwin"
	# 	build_target "aarch64-apple-darwin"
	# 	;;
	# macos-x86_64)
	# 	build_target "x86_64-apple-darwin"
	# 	;;
	# macos-aarch64)
	# 	build_target "aarch64-apple-darwin"
	# 	;;
	# windows)
	# 	build_target "x86_64-pc-windows-gnu"
	# 	;;
	# windows-x86_64)
	# 	build_target "x86_64-pc-windows-gnu"
	# 	;;
	*)
		log_error "Unknown target: $target"
		log_error "Run '$0 --help' to see available targets"
		exit 1
		;;
	esac
}

# Main function
main() {
	log_info "Starting cross-compilation for $PROJECT_NAME v$VERSION"

	parse_args "$@"
	check_dependencies

	local start_time
	start_time=$(date +%s)

	build_targets "$TARGET"

	local end_time
	end_time=$(date +%s)
	local duration=$((end_time - start_time))

	log_success "Cross-compilation completed in ${duration}s"
	log_success "All targets built successfully!"

	if [[ "${CREATE_DIST:-false}" == "true" ]]; then
		log_success "Distribution archives created in: $DIST_DIR"
		if [[ -d "$DIST_DIR" ]]; then
			ls -la "$DIST_DIR"
		fi
	fi
}

# Execute main function
main "$@"
