# Arch Linux packaging test environment
# Tests PKGBUILD package building and installation
#
# Build: docker build -f packaging/docker-test/arch.Dockerfile -t dotman-test-arch .
# Run:   docker run --rm dotman-test-arch

FROM archlinux:base-devel AS builder

# Update system and install dependencies
RUN pacman -Syu --noconfirm && \
    pacman -S --noconfirm \
        base-devel \
        curl \
        git \
        help2man \
        pkg-config \
        zstd \
    && pacman -Scc --noconfirm

# Create build user (makepkg refuses to run as root)
RUN useradd -m builder && \
    echo "builder ALL=(ALL) NOPASSWD: ALL" >> /etc/sudoers

# Install Rust for builder user
USER builder
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
ENV PATH="/home/builder/.cargo/bin:${PATH}"

WORKDIR /home/builder

# Copy source
COPY --chown=builder:builder . /home/builder/dotman

WORKDIR /home/builder/dotman

# Get version
RUN VERSION=$(grep '^version = ' Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/') && \
    echo "${VERSION}" > /tmp/version

# Create local PKGBUILD for testing
# Note: We use source=() and skip build() since we build from local copy
RUN VERSION=$(cat /tmp/version) && cat > PKGBUILD << PKGBUILD_EOF
# Maintainer: Utsav Balar <utsavbalar1231@gmail.com>

pkgname=dotman
pkgver=${VERSION}
pkgrel=1
pkgdesc="High-performance dotfiles manager with git-like semantics"
arch=('x86_64' 'aarch64')
url="https://github.com/UtsavBalar1231/dotman"
license=('MIT')
depends=('gcc-libs' 'glibc')
makedepends=('help2man')
provides=('dotman')
conflicts=('dotman-git')

# No source - building from local directory
source=()
md5sums=()

build() {
    cd /home/builder/dotman
    # Build zstd from source to avoid linking issues with rustup
    export ZSTD_SYS_USE_PKG_CONFIG=0
    cargo build --release --all-features
}

package() {
    cd /home/builder/dotman

    # Install binary
    install -Dm755 target/release/dot "\${pkgdir}/usr/bin/dot"

    # Install documentation
    install -Dm644 README.md "\${pkgdir}/usr/share/doc/\${pkgname}/README.md"

    # Generate and install shell completions
    mkdir -p "\${pkgdir}/usr/share/bash-completion/completions"
    ./target/release/dot completion bash > "\${pkgdir}/usr/share/bash-completion/completions/dot"

    mkdir -p "\${pkgdir}/usr/share/zsh/site-functions"
    ./target/release/dot completion zsh > "\${pkgdir}/usr/share/zsh/site-functions/_dot"

    mkdir -p "\${pkgdir}/usr/share/fish/vendor_completions.d"
    ./target/release/dot completion fish > "\${pkgdir}/usr/share/fish/vendor_completions.d/dot.fish"

    # Generate and install man page
    mkdir -p "\${pkgdir}/usr/share/man/man1"
    help2man --no-info --name="high-performance dotfiles manager" \
        --version-string="\${pkgver}" ./target/release/dot > "\${pkgdir}/usr/share/man/man1/dot.1"
}
PKGBUILD_EOF

# Build package using makepkg
RUN makepkg -sf --noconfirm

# Move package to accessible location (exclude debug package)
RUN cp dotman-0*.pkg.tar.zst /tmp/dotman.pkg.tar.zst

# Test installation stage
FROM archlinux:base AS test

# Copy the built package
COPY --from=builder /tmp/dotman.pkg.tar.zst /tmp/

# Remove NoExtract for man pages (base image excludes them by default)
RUN sed -i 's/^NoExtract.*man.*info.*$/# &/' /etc/pacman.conf

# Install the package
RUN pacman -Syu --noconfirm && \
    pacman -U --noconfirm /tmp/dotman.pkg.tar.zst

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
    test -f /usr/share/zsh/site-functions/_dot && \
    test -f /usr/share/fish/vendor_completions.d/dot.fish && \
    echo "Shell completions installed correctly!"

# Verify man page (makepkg compresses man pages to .gz)
RUN test -f /usr/share/man/man1/dot.1.gz && \
    echo "Man page installed correctly!"

# Final verification
CMD ["dot", "--version"]
