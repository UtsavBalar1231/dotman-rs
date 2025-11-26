# Alpine Linux packaging test environment
# Tests static musl binary building and installation
#
# Build: docker build -f packaging/docker-test/alpine.Dockerfile -t dotman-test-alpine .
# Run:   docker run --rm dotman-test-alpine

# Use Debian-based Rust for building (proc-macros work properly)
FROM rust:bookworm AS builder

# Install build dependencies including musl toolchain
RUN apt-get update && apt-get install -y --no-install-recommends \
    git \
    help2man \
    musl-tools \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

# Add musl target
RUN rustup target add x86_64-unknown-linux-musl

WORKDIR /build

# Copy source
COPY . .

# Build static binary with musl target
ENV RUSTFLAGS="-C target-feature=+crt-static -C opt-level=3"
RUN cargo build --release --target x86_64-unknown-linux-musl --all-features

# Verify it's statically linked
RUN file target/x86_64-unknown-linux-musl/release/dot | grep -i static || echo "Warning: Binary may not be fully static"

# Generate shell completions
RUN mkdir -p completions && \
    ./target/x86_64-unknown-linux-musl/release/dot completion bash > completions/dot.bash && \
    ./target/x86_64-unknown-linux-musl/release/dot completion zsh > completions/_dot && \
    ./target/x86_64-unknown-linux-musl/release/dot completion fish > completions/dot.fish

# Get version
RUN VERSION=$(grep '^version = ' Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/') && \
    echo "${VERSION}" > /tmp/version

# Generate man page
RUN VERSION=$(cat /tmp/version) && \
    help2man --no-info --name="high-performance dotfiles manager" \
        --version-string="${VERSION}" ./target/x86_64-unknown-linux-musl/release/dot > dot.1

# Create installation structure
RUN VERSION=$(cat /tmp/version) && \
    mkdir -p /pkg/usr/bin && \
    mkdir -p /pkg/usr/share/doc/dotman && \
    mkdir -p /pkg/usr/share/man/man1 && \
    mkdir -p /pkg/usr/share/bash-completion/completions && \
    mkdir -p /pkg/usr/share/zsh/site-functions && \
    mkdir -p /pkg/usr/share/fish/vendor_completions.d && \
    # Binary
    install -m755 target/x86_64-unknown-linux-musl/release/dot /pkg/usr/bin/dot && \
    # Documentation
    install -m644 README.md /pkg/usr/share/doc/dotman/ && \
    # Shell completions
    install -m644 completions/dot.bash /pkg/usr/share/bash-completion/completions/dot && \
    install -m644 completions/_dot /pkg/usr/share/zsh/site-functions/_dot && \
    install -m644 completions/dot.fish /pkg/usr/share/fish/vendor_completions.d/dot.fish && \
    # Man page
    install -m644 dot.1 /pkg/usr/share/man/man1/dot.1

# Create tarball for distribution
RUN VERSION=$(cat /tmp/version) && \
    cd /pkg && \
    tar czf /tmp/dotman-${VERSION}-alpine-x86_64.tar.gz .

# Test installation stage - use alpine for minimal runtime
FROM alpine:3.21 AS test

# Copy installation files directly
COPY --from=builder /pkg/usr/bin/dot /usr/bin/dot
COPY --from=builder /pkg/usr/share/bash-completion/completions/dot /usr/share/bash-completion/completions/dot
COPY --from=builder /pkg/usr/share/zsh/site-functions/_dot /usr/share/zsh/site-functions/_dot
COPY --from=builder /pkg/usr/share/fish/vendor_completions.d/dot.fish /usr/share/fish/vendor_completions.d/dot.fish
COPY --from=builder /pkg/usr/share/man/man1/dot.1 /usr/share/man/man1/dot.1
COPY --from=builder /pkg/usr/share/doc/dotman/README.md /usr/share/doc/dotman/README.md

# Create test user
RUN adduser -D -s /bin/sh testuser
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

# Verify man page
RUN test -f /usr/share/man/man1/dot.1 && \
    echo "Man page installed correctly!"

# Final verification
CMD ["dot", "--version"]
