# dotman - Rust Build System
# Professional build automation for dotfiles manager

default:
    @just --list

# Build Targets
build:
    cargo build --all-features

build-release:
    cargo build --release --all-features

build-optimized:
    #!/usr/bin/env bash
    set -euo pipefail
    export RUSTFLAGS="-C target-cpu=native -C opt-level=3"
    cargo build --release --all-features

build-all:
    just cross-compile all

cross-compile target="all":
    ./scripts/cross-compile.sh --dist {{ target }}

# Test Suite
test:
    cargo test --all-features

test-verbose:
    cargo test --all-features -- --nocapture

test-unit:
    cargo test --lib --all-features

test-integration:
    cargo test --test integration_test

test-property:
    cargo test --test property_based_tests

test-coverage:
    #!/usr/bin/env bash
    set -euo pipefail
    cargo llvm-cov --all-features --workspace --html
    echo "Coverage report: target/llvm-cov/html/index.html"

test-sequential:
    cargo test --all-features -- --test-threads=1

# Code Quality
fmt:
    cargo fmt --all

fmt-check:
    cargo fmt --all -- --check

lint:
    cargo clippy --all-targets --all-features -- \
        -D warnings \
        -W clippy::nursery \
        -W clippy::pedantic

lint-strict:
    #!/usr/bin/env bash
    cargo clippy --all-targets --all-features -- \
        -D warnings \
        -W clippy::arithmetic_side_effects \
        -W clippy::expect_used \
        -W clippy::float_arithmetic \
        -W clippy::indexing_slicing \
        -W clippy::mem_forget \
        -W clippy::nursery \
        -W clippy::panic \
        -W clippy::pedantic \
        -W clippy::unwrap_used

fix:
    cargo fix --allow-dirty --allow-staged --all-features

# Security & Dependencies
audit:
    cargo audit

unused-deps:
    cargo machete

security: audit

# Performance
bench:
    cargo bench

bench-name name:
    cargo bench {{ name }}

profile-perf:
    #!/usr/bin/env bash
    set -euo pipefail
    cargo build --profile release-with-debug
    perf record --call-graph=dwarf target/release-with-debug/dot --help
    perf report

profile-flame:
    cargo flamegraph --bin dot -- --help

profile-memory:
    #!/usr/bin/env bash
    set -euo pipefail
    cargo build --profile release-with-debug
    valgrind --tool=massif target/release-with-debug/dot --help

# Documentation
docs:
    cargo doc --no-deps --all-features --open

docs-check:
    RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features

# Man Pages
generate-manpages:
    cargo run -p xtask -- generate-man-pages

install-manpages: generate-manpages
    #!/usr/bin/env bash
    set -euo pipefail
    echo "Installing man pages to /usr/share/man/man1/ (requires sudo)..."
    sudo install -Dm644 man/*.1 -t /usr/share/man/man1/
    sudo mandb
    echo "Man pages installed successfully!"

# Development Tools
watch:
    cargo watch -x "build --all-features"

watch-test:
    cargo watch -x "test --all-features"

setup:
    #!/usr/bin/env bash
    set -euo pipefail
    echo "Installing required development tools..."
    cargo install cargo-watch
    cargo install cargo-audit
    cargo install cargo-machete
    cargo install cargo-llvm-cov
    cargo install flamegraph
    echo "All tools installed successfully!"

# Installation
install:
    cargo install --path . --force

install-crates:
    cargo install dotman

uninstall:
    cargo uninstall dotman

# Packaging
package-debian:
    #!/usr/bin/env bash
    set -euo pipefail

    # Build release binary
    cargo build --release

    # Create debian package structure
    VERSION=$(grep '^version = ' Cargo.toml | sed 's/.*"\(.*\)".*/\1/')
    ARCH=$(dpkg --print-architecture)
    PKG_NAME="dotman_${VERSION}-1_${ARCH}"
    PKG_DIR="dist/debian/${PKG_NAME}"

    rm -rf "${PKG_DIR}"
    mkdir -p "${PKG_DIR}/DEBIAN"
    mkdir -p "${PKG_DIR}/usr/bin"
    mkdir -p "${PKG_DIR}/usr/share/doc/dotman"
    mkdir -p "${PKG_DIR}/usr/share/man/man1"

    # Copy binary
    cp target/release/dot "${PKG_DIR}/usr/bin/"
    chmod 755 "${PKG_DIR}/usr/bin/dot"

    # Copy documentation
    cp README.md "${PKG_DIR}/usr/share/doc/dotman/"
    cp LICENSE "${PKG_DIR}/usr/share/doc/dotman/"

    # Generate man page
    help2man --no-info --name="high-performance dotfiles manager" \
        --output="${PKG_DIR}/usr/share/man/man1/dot.1" \
        target/release/dot
    gzip -9 "${PKG_DIR}/usr/share/man/man1/dot.1"

    # Create control file
    echo "Package: dotman" > "${PKG_DIR}/DEBIAN/control"
    echo "Version: ${VERSION}-1" >> "${PKG_DIR}/DEBIAN/control"
    echo "Section: utils" >> "${PKG_DIR}/DEBIAN/control"
    echo "Priority: optional" >> "${PKG_DIR}/DEBIAN/control"
    echo "Architecture: ${ARCH}" >> "${PKG_DIR}/DEBIAN/control"
    echo "Maintainer: UtsavBalar1231 <utsavbalar1231@gmail.com>" >> "${PKG_DIR}/DEBIAN/control"
    echo "Depends: libc6 (>= 2.31), libgcc-s1" >> "${PKG_DIR}/DEBIAN/control"
    echo "Suggests: git" >> "${PKG_DIR}/DEBIAN/control"
    echo "Homepage: https://github.com/UtsavBalar1231/dotman-rs" >> "${PKG_DIR}/DEBIAN/control"
    echo "Description: High-performance dotfiles manager" >> "${PKG_DIR}/DEBIAN/control"
    echo " A blazingly fast dotfiles manager with git-like semantics," >> "${PKG_DIR}/DEBIAN/control"
    echo " built with Rust for maximum performance and reliability." >> "${PKG_DIR}/DEBIAN/control"

    # Build the package
    dpkg-deb --build --root-owner-group "${PKG_DIR}"
    mv "dist/debian/${PKG_NAME}.deb" dist/
    echo "Created: dist/${PKG_NAME}.deb"

package-rpm:
    #!/usr/bin/env bash
    set -euo pipefail

    # Check for required tools
    if ! command -v rpmbuild >/dev/null 2>&1; then
        echo "Error: rpmbuild not found. This command requires an RPM-based distribution."
        echo "Install with: sudo dnf install rpm-build (Fedora) or sudo yum install rpm-build (RHEL/CentOS)"
        echo "On other systems, you can use Docker: docker run --rm -v \$(pwd):/build fedora:latest"
        exit 1
    fi

    # Build release binary
    cargo build --release

    # Setup rpmbuild directory structure
    RPMBUILD_DIR="${HOME}/rpmbuild"
    mkdir -p "${RPMBUILD_DIR}"/{BUILD,RPMS,SOURCES,SPECS,SRPMS}

    # Get version
    VERSION=$(grep '^version = ' Cargo.toml | sed 's/.*"\(.*\)".*/\1/')

    # Create source tarball with correct directory structure
    PROJECT_DIR=$(pwd)
    cd ..
    tar czf "${RPMBUILD_DIR}/SOURCES/dotman-${VERSION}.tar.gz" \
        --exclude=dotman/.git \
        --exclude=dotman/target \
        --exclude=dotman/dist \
        --transform "s,^dotman,dotman-${VERSION}," \
        dotman
    cd "${PROJECT_DIR}"

    # Copy spec file
    # Use simpler spec on non-Fedora systems
    if [ -f /etc/fedora-release ] || [ -f /etc/redhat-release ]; then
        cp packaging/rpm/dotman.spec "${RPMBUILD_DIR}/SPECS/"
        rpmbuild -ba "${RPMBUILD_DIR}/SPECS/dotman.spec"
    else
        # Use simpler spec for other systems (like Arch with rpmbuild)
        cp packaging/rpm/dotman-simple.spec "${RPMBUILD_DIR}/SPECS/dotman.spec"
        rpmbuild -bb "${RPMBUILD_DIR}/SPECS/dotman.spec"
    fi

    # Copy to dist
    mkdir -p dist
    cp "${RPMBUILD_DIR}/RPMS/"*"/dotman-${VERSION}"*.rpm dist/ 2>/dev/null || true
    cp "${RPMBUILD_DIR}/SRPMS/dotman-${VERSION}"*.src.rpm dist/ 2>/dev/null || true

    echo "RPM packages created in dist/"

package-arch:
    #!/usr/bin/env bash
    set -euo pipefail

    # Check for required tools
    if ! command -v makepkg >/dev/null 2>&1; then
        echo "Error: makepkg not found. This command requires Arch Linux or an Arch-based distribution."
        echo "On other systems, you can use Docker: docker run --rm -v \$(pwd):/build archlinux:latest"
        exit 1
    fi

    # Build release binary
    cargo build --release

    # Get version
    VERSION=$(grep '^version = ' Cargo.toml | sed 's/.*"\(.*\)".*/\1/')

    # Create build directory
    BUILD_DIR="dist/arch/build"
    rm -rf "${BUILD_DIR}"
    mkdir -p "${BUILD_DIR}"

    # Create a proper source tarball
    echo "Creating source tarball..."
    tar czf "${BUILD_DIR}/dotman-${VERSION}.tar.gz" \
        --exclude=.git \
        --exclude=target \
        --exclude=dist \
        --exclude='*.swp' \
        --transform "s,^,dotman-${VERSION}/," \
        .

    # Copy PKGBUILD
    cp packaging/arch/PKGBUILD "${BUILD_DIR}/"

    # Update PKGBUILD to use local source tarball
    cd "${BUILD_DIR}"
    sed -i "s|source=(.*)|source=('dotman-${VERSION}.tar.gz')|" PKGBUILD
    sed -i "s|sha256sums=(.*)|sha256sums=('SKIP')|" PKGBUILD

    # Add environment variable to force bundled zstd build
    sed -i '/^build() {/a\    export ZSTD_SYS_USE_PKG_CONFIG=0' PKGBUILD

    # Build package
    makepkg -f

    # Copy to dist
    cd -
    mkdir -p dist
    cp "${BUILD_DIR}/"*.pkg.tar.* dist/ 2>/dev/null || true

    echo "Arch package created in dist/"

package-alpine:
    #!/usr/bin/env bash
    set -euo pipefail

    # Build static binary for Alpine
    cargo build --release --target x86_64-unknown-linux-musl

    # Get version
    VERSION=$(grep '^version = ' Cargo.toml | sed 's/.*"\(.*\)".*/\1/')

    # Create Alpine package structure
    PKG_DIR="dist/alpine/dotman-${VERSION}"
    rm -rf "${PKG_DIR}"
    mkdir -p "${PKG_DIR}/usr/bin"
    mkdir -p "${PKG_DIR}/usr/share/doc/dotman"

    # Copy static binary
    cp target/x86_64-unknown-linux-musl/release/dot "${PKG_DIR}/usr/bin/"
    chmod 755 "${PKG_DIR}/usr/bin/dot"

    # Copy documentation
    cp README.md LICENSE "${PKG_DIR}/usr/share/doc/dotman/"

    # Create tarball
    cd dist/alpine
    tar czf "dotman-${VERSION}-alpine.tar.gz" "dotman-${VERSION}"
    cd -
    mv "dist/alpine/dotman-${VERSION}-alpine.tar.gz" dist/

    echo "Alpine package created: dist/dotman-${VERSION}-alpine.tar.gz"

# Containers
docker-build:
    docker build --network=host -f containers/Dockerfile -t dotman:latest .

docker-build-alpine:
    docker build --network=host -f containers/Dockerfile.alpine -t dotman:alpine .

docker-run:
    docker run --rm -it dotman:latest

docker-test:
    #!/usr/bin/env bash
    set -euo pipefail
    just docker-build-alpine
    echo "Testing Alpine container..."
    docker run --rm dotman:alpine --version

# Release Management
release-prepare version:
    #!/usr/bin/env bash
    set -euo pipefail

    # Update version in all files
    sed -i 's/^version = ".*"/version = "{{ version }}"/' Cargo.toml
    sed -i 's/^pkgver=.*/pkgver={{ version }}/' packaging/arch/PKGBUILD
    sed -i 's/^Version:.*/Version: {{ version }}/' packaging/rpm/dotman.spec
    sed -i "s/^Version:.*/Version: {{ version }}-1/" packaging/debian/control

    # Update Cargo.lock
    cargo update -p dotman

    # Run quality checks
    just test
    just lint
    just security

release-artifacts:
    #!/usr/bin/env bash
    set -euo pipefail

    echo "Building release artifacts..."

    # Clean previous artifacts
    rm -rf dist
    mkdir -p dist

    # Build for all platforms
    ./scripts/cross-compile.sh --dist all

    # Build packages
    just package-debian
    just package-rpm
    just package-arch
    just docker-build-alpine

    # Generate checksums
    cd dist
    sha256sum *.tar.gz *.zip *.deb *.rpm *.pkg.tar.* > checksums.sha256
    md5sum *.tar.gz *.zip *.deb *.rpm *.pkg.tar.* > checksums.md5

    echo "Release artifacts created in dist/"
    ls -la dist/

# Cleanup
clean:
    cargo clean
    rm -rf dist/

# Quality Gates
pre-commit: fmt-check lint test-unit

ci: fmt-check lint test security docs-check

quality: fmt-check lint test security bench docs-check

dev: clean build-optimized test lint docs
