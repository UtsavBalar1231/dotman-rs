#!/usr/bin/env bash
# Build and test all package formats in Docker
#
# Usage:
#   ./build-all.sh           # Build all formats
#   ./build-all.sh debian    # Build specific format
#   ./build-all.sh --help    # Show help

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Available package formats
FORMATS=(debian arch alpine fedora)

usage() {
    echo "Usage: $0 [OPTIONS] [FORMAT...]"
    echo ""
    echo "Build and test dotman packages in Docker containers."
    echo ""
    echo "Formats:"
    echo "  debian    Build and test Debian .deb package"
    echo "  arch      Build and test Arch Linux .pkg.tar.zst package"
    echo "  alpine    Build and test Alpine static binary"
    echo "  fedora    Build and test Fedora/RHEL .rpm package"
    echo ""
    echo "Options:"
    echo "  --help    Show this help message"
    echo "  --no-cache  Build without Docker cache"
    echo "  --parallel  Build all formats in parallel"
    echo ""
    echo "Examples:"
    echo "  $0                    # Build all formats sequentially"
    echo "  $0 debian arch        # Build only Debian and Arch"
    echo "  $0 --parallel         # Build all formats in parallel"
    echo "  $0 --no-cache debian  # Build Debian without cache"
}

log_info() {
    echo -e "${BLUE}[INFO]${NC} $*"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $*"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $*"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $*"
}

build_format() {
    local format="$1"
    local no_cache="${2:-false}"
    local dockerfile="${SCRIPT_DIR}/${format}.Dockerfile"
    local image_name="dotman-test-${format}"

    if [[ ! -f "${dockerfile}" ]]; then
        log_error "Dockerfile not found: ${dockerfile}"
        return 1
    fi

    log_info "Building ${format} package..."

    local cache_arg=""
    if [[ "${no_cache}" == "true" ]]; then
        cache_arg="--no-cache"
    fi

    # Build the Docker image
    if docker build \
        ${cache_arg} \
        --network=host \
        -f "${dockerfile}" \
        -t "${image_name}" \
        "${PROJECT_ROOT}"; then
        log_success "${format} package built successfully!"

        # Run the container to verify
        log_info "Verifying ${format} installation..."
        if docker run --rm "${image_name}" dot --version; then
            log_success "${format} verification passed!"
            return 0
        else
            log_error "${format} verification failed!"
            return 1
        fi
    else
        log_error "${format} build failed!"
        return 1
    fi
}

build_parallel() {
    local no_cache="$1"
    shift
    local formats=("$@")
    local pids=()
    local results=()

    log_info "Building ${#formats[@]} formats in parallel..."

    for format in "${formats[@]}"; do
        (build_format "${format}" "${no_cache}") &
        pids+=($!)
    done

    # Wait for all builds and collect results
    local failed=0
    for i in "${!pids[@]}"; do
        if wait "${pids[$i]}"; then
            results+=("${formats[$i]}:success")
        else
            results+=("${formats[$i]}:failed")
            failed=1
        fi
    done

    # Print summary
    echo ""
    log_info "Build Summary:"
    for result in "${results[@]}"; do
        local format="${result%%:*}"
        local status="${result##*:}"
        if [[ "${status}" == "success" ]]; then
            log_success "  ${format}: passed"
        else
            log_error "  ${format}: failed"
        fi
    done

    return ${failed}
}

build_sequential() {
    local no_cache="$1"
    shift
    local formats=("$@")
    local failed=0
    local results=()

    for format in "${formats[@]}"; do
        echo ""
        echo "========================================"
        echo "Building: ${format}"
        echo "========================================"

        if build_format "${format}" "${no_cache}"; then
            results+=("${format}:success")
        else
            results+=("${format}:failed")
            failed=1
        fi
    done

    # Print summary
    echo ""
    echo "========================================"
    log_info "Build Summary:"
    echo "========================================"
    for result in "${results[@]}"; do
        local format="${result%%:*}"
        local status="${result##*:}"
        if [[ "${status}" == "success" ]]; then
            log_success "  ${format}: passed"
        else
            log_error "  ${format}: failed"
        fi
    done

    return ${failed}
}

main() {
    local no_cache="false"
    local parallel="false"
    local formats_to_build=()

    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --help|-h)
                usage
                exit 0
                ;;
            --no-cache)
                no_cache="true"
                shift
                ;;
            --parallel|-p)
                parallel="true"
                shift
                ;;
            *)
                # Check if it's a valid format
                if [[ " ${FORMATS[*]} " =~ " $1 " ]]; then
                    formats_to_build+=("$1")
                else
                    log_error "Unknown format or option: $1"
                    usage
                    exit 1
                fi
                shift
                ;;
        esac
    done

    # Default to all formats if none specified
    if [[ ${#formats_to_build[@]} -eq 0 ]]; then
        formats_to_build=("${FORMATS[@]}")
    fi

    # Check Docker is available
    if ! command -v docker &> /dev/null; then
        log_error "Docker is not installed or not in PATH"
        exit 1
    fi

    # Check Docker daemon is running
    if ! docker info &> /dev/null; then
        log_error "Docker daemon is not running"
        exit 1
    fi

    log_info "Building packages for: ${formats_to_build[*]}"
    log_info "Project root: ${PROJECT_ROOT}"

    if [[ "${parallel}" == "true" ]]; then
        build_parallel "${no_cache}" "${formats_to_build[@]}"
    else
        build_sequential "${no_cache}" "${formats_to_build[@]}"
    fi
}

main "$@"
