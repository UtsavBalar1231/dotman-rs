# syntax=docker/dockerfile:1.4
# Fedora/RPM packaging test environment
# Tests .rpm package building and installation
#
# Build: docker build -f packaging/docker-test/fedora.Dockerfile -t dotman-test-rpm .
# Run:   docker run --rm dotman-test-rpm

FROM fedora:41 AS builder

# Install build dependencies (use rustup for Rust 1.85+ needed for edition 2024)
RUN dnf install -y \
    curl \
    gcc \
    git \
    help2man \
    rpm-build \
    rpmdevtools \
    && dnf clean all

# Install Rust via rustup (need 1.85+ for edition 2024)
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
ENV PATH="/root/.cargo/bin:${PATH}"

WORKDIR /build

# Copy source
COPY . .

# Build release binary
RUN cargo build --release --all-features

# Generate shell completions
RUN mkdir -p completions && \
    ./target/release/dot completion bash > completions/dot.bash && \
    ./target/release/dot completion zsh > completions/_dot && \
    ./target/release/dot completion fish > completions/dot.fish

# Get version
RUN VERSION=$(grep '^version = ' Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/') && \
    echo "${VERSION}" > /tmp/version

# Generate man page
RUN VERSION=$(cat /tmp/version) && \
    help2man --no-info --name="high-performance dotfiles manager" \
        --version-string="${VERSION}" ./target/release/dot > dot.1

# Setup rpmbuild directory
RUN rpmdev-setuptree

# Create simplified spec file that works without rust-packaging
RUN <<SPEC_SCRIPT
#!/bin/bash
set -e
VERSION=$(cat /tmp/version)
cat > ~/rpmbuild/SPECS/dotman.spec <<'SPEC_EOF'
Name:           dotman
Version:        VERSION_PLACEHOLDER
Release:        1%{?dist}
Summary:        High-performance dotfiles manager with git-like semantics

License:        MIT
URL:            https://github.com/UtsavBalar1231/dotman

# No Source - we build from local directory

BuildRequires:  gcc
BuildRequires:  help2man

%description
dotman is a blazingly fast dotfiles manager built in Rust with
SIMD acceleration, parallel processing, and content deduplication.

Features:
- Git-like interface with familiar commands
- SIMD-accelerated operations
- Parallel file processing
- Content-addressed storage with deduplication
- Zstd compression

%prep
# Nothing to prep - using pre-built binary

%build
# Already built

%install
# Binary
install -Dm755 /build/target/release/dot %{buildroot}%{_bindir}/dot

# Documentation
install -Dm644 /build/README.md %{buildroot}%{_docdir}/%{name}/README.md

# Shell completions
install -Dm644 /build/completions/dot.bash %{buildroot}%{_datadir}/bash-completion/completions/dot
install -Dm644 /build/completions/_dot %{buildroot}%{_datadir}/zsh/site-functions/_dot
install -Dm644 /build/completions/dot.fish %{buildroot}%{_datadir}/fish/vendor_completions.d/dot.fish

# Man page
install -Dm644 /build/dot.1 %{buildroot}%{_mandir}/man1/dot.1

%files
%{_bindir}/dot
%{_docdir}/%{name}/README.md
%{_datadir}/bash-completion/completions/dot
%{_datadir}/zsh/site-functions/_dot
%{_datadir}/fish/vendor_completions.d/dot.fish
%{_mandir}/man1/dot.1*

%changelog
* %(date "+%a %b %d %Y") Utsav Balar <utsavbalar1231@gmail.com> - VERSION_PLACEHOLDER-1
- Package built for testing
SPEC_EOF
SPEC_SCRIPT

# Update version in spec
RUN VERSION=$(cat /tmp/version) && \
    sed -i "s/VERSION_PLACEHOLDER/${VERSION}/g" ~/rpmbuild/SPECS/dotman.spec

# Build the RPM (binary only, no source package)
RUN rpmbuild -bb ~/rpmbuild/SPECS/dotman.spec

# Copy RPM to accessible location
RUN cp ~/rpmbuild/RPMS/*/dotman-*.rpm /tmp/dotman.rpm

# Test installation stage
FROM fedora:41 AS test

# Copy the built package
COPY --from=builder /tmp/dotman.rpm /tmp/

# Remove nodocs flag to allow man page installation
RUN sed -i 's/^tsflags=nodocs$/# &/' /etc/dnf/dnf.conf

# Install the package
RUN dnf install -y /tmp/dotman.rpm && \
    dnf clean all

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

# Verify man page (RPM compresses man pages to .gz)
RUN test -f /usr/share/man/man1/dot.1.gz && \
    echo "Man page installed correctly!"

# Final verification
CMD ["dot", "--version"]
