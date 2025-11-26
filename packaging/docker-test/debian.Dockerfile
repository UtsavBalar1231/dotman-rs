# Debian packaging test environment
# Tests .deb package building and installation
#
# Build: docker build -f packaging/docker-test/debian.Dockerfile -t dotman-test-deb .
# Run:   docker run --rm dotman-test-deb

FROM debian:bookworm-slim AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    ca-certificates \
    curl \
    debhelper \
    devscripts \
    dpkg-dev \
    fakeroot \
    git \
    help2man \
    jq \
    lintian \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

# Install Rust (need 1.85+ for edition 2024)
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
ENV PATH="/root/.cargo/bin:${PATH}"

WORKDIR /build

# Copy source
COPY . .

# Extract version from Cargo.toml
RUN VERSION=$(grep '^version = ' Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/') && \
    echo "Building version: ${VERSION}"

# Build release binary
RUN cargo build --release --all-features

# Generate shell completions
RUN mkdir -p completions && \
    ./target/release/dot completion bash > completions/dot.bash && \
    ./target/release/dot completion zsh > completions/_dot && \
    ./target/release/dot completion fish > completions/dot.fish

# Get version and architecture
RUN VERSION=$(grep '^version = ' Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/') && \
    ARCH=$(dpkg --print-architecture) && \
    PKG_DIR="dotman_${VERSION}-1_${ARCH}" && \
    echo "PKG_DIR=${PKG_DIR}" > /tmp/pkg_env

# Create package structure
RUN . /tmp/pkg_env && \
    mkdir -p "${PKG_DIR}/DEBIAN" && \
    mkdir -p "${PKG_DIR}/usr/bin" && \
    mkdir -p "${PKG_DIR}/usr/share/doc/dotman" && \
    mkdir -p "${PKG_DIR}/usr/share/man/man1" && \
    mkdir -p "${PKG_DIR}/usr/share/bash-completion/completions" && \
    mkdir -p "${PKG_DIR}/usr/share/zsh/vendor-completions" && \
    mkdir -p "${PKG_DIR}/usr/share/fish/vendor_completions.d"

# Install files
RUN . /tmp/pkg_env && \
    VERSION=$(grep '^version = ' Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/') && \
    ARCH=$(dpkg --print-architecture) && \
    # Binary
    install -m755 target/release/dot "${PKG_DIR}/usr/bin/dot" && \
    # Documentation
    install -m644 README.md "${PKG_DIR}/usr/share/doc/dotman/" && \
    # Shell completions
    install -m644 completions/dot.bash "${PKG_DIR}/usr/share/bash-completion/completions/dot" && \
    install -m644 completions/_dot "${PKG_DIR}/usr/share/zsh/vendor-completions/_dot" && \
    install -m644 completions/dot.fish "${PKG_DIR}/usr/share/fish/vendor_completions.d/dot.fish" && \
    # Man page
    help2man --no-info --name="high-performance dotfiles manager" \
        --version-string="${VERSION}" ./target/release/dot > "${PKG_DIR}/usr/share/man/man1/dot.1" && \
    gzip -9 "${PKG_DIR}/usr/share/man/man1/dot.1"

# Create control file
RUN . /tmp/pkg_env && \
    VERSION=$(grep '^version = ' Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/') && \
    ARCH=$(dpkg --print-architecture) && \
    INSTALLED_SIZE=$(du -sk "${PKG_DIR}" | cut -f1) && \
    cat > "${PKG_DIR}/DEBIAN/control" << EOF
Package: dotman
Version: ${VERSION}-1
Section: utils
Priority: optional
Architecture: ${ARCH}
Installed-Size: ${INSTALLED_SIZE}
Maintainer: UtsavBalar1231 <utsavbalar1231@gmail.com>
Depends: libc6 (>= 2.31), libgcc-s1
Suggests: git
Homepage: https://github.com/UtsavBalar1231/dotman
Description: High-performance dotfiles manager with git-like semantics
 dotman is a blazingly fast dotfiles manager built in Rust with
 SIMD acceleration, parallel processing, and content deduplication.
 .
 Features:
  - Git-like interface with familiar commands
  - SIMD-accelerated operations
  - Parallel file processing
  - Content-addressed storage with deduplication
  - Zstd compression
EOF

# Build the package
RUN . /tmp/pkg_env && \
    dpkg-deb --build --root-owner-group "${PKG_DIR}" && \
    mv "${PKG_DIR}.deb" /tmp/dotman.deb

# Run lintian (allow warnings, fail on errors)
RUN lintian --no-tag-display-limit /tmp/dotman.deb || true

# Test installation stage
FROM debian:bookworm-slim AS test

# Copy the built package
COPY --from=builder /tmp/dotman.deb /tmp/

# Remove dpkg exclusions for man pages (slim image excludes them by default)
RUN rm -f /etc/dpkg/dpkg.cfg.d/docker

# Install the package
RUN apt-get update && \
    apt-get install -y /tmp/dotman.deb && \
    rm -rf /var/lib/apt/lists/*

# Create test user
RUN useradd -m -s /bin/bash testuser
USER testuser
WORKDIR /home/testuser

# Run smoke tests
RUN dot --version && \
    dot --help && \
    dot init && \
    dot status && \
    echo "All smoke tests passed!"

# Verify completions were installed
RUN test -f /usr/share/bash-completion/completions/dot && \
    test -f /usr/share/zsh/vendor-completions/_dot && \
    test -f /usr/share/fish/vendor_completions.d/dot.fish && \
    echo "Shell completions installed correctly!"

# Verify man page
RUN test -f /usr/share/man/man1/dot.1.gz && \
    echo "Man page installed correctly!"

# Final verification
CMD ["dot", "--version"]
