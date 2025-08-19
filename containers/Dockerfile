# Multi-stage Dockerfile for dotman
FROM rustlang/rust:nightly-bookworm AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    help2man \
    && rm -rf /var/lib/apt/lists/*

# Create build directory
WORKDIR /usr/src/app

# Copy manifest files first
COPY Cargo.toml Cargo.lock ./

# Copy source code
COPY src ./src

# Copy tests
COPY tests ./tests

# Copy benches
COPY benches ./benches

# Copy README.md
COPY README.md ./README.md

# Build the application
RUN cargo build --release --all-features

# Verify binary works
RUN ./target/release/dot --version

# Runtime stage
FROM debian:bookworm-slim AS runtime

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    git \
    && rm -rf /var/lib/apt/lists/*

# Create app user
RUN useradd -m -u 1000 app

# Create dotman directories
RUN mkdir -p /home/app/.dotman /home/app/.config/dotman && \
    chown -R app:app /home/app

# Copy binary and documentation
COPY --from=builder /usr/src/app/target/release/dot /usr/local/bin/dot
COPY --from=builder /usr/src/app/README.md /usr/share/doc/dotman/

# Verify binary
RUN dot --version

USER app
WORKDIR /home/app

ENV DOTMAN_CONFIG_PATH=/home/app/.config/dotman/config

HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD dot --version || exit 1

CMD ["dot", "--help"]

# Metadata
LABEL org.opencontainers.image.title="dotman"
LABEL org.opencontainers.image.description="High-performance dotfiles manager"
LABEL org.opencontainers.image.url="https://github.com/UtsavBalar1231/dotman-rs"
LABEL org.opencontainers.image.source="https://github.com/UtsavBalar1231/dotman-rs"
LABEL org.opencontainers.image.licenses="MIT"
LABEL org.opencontainers.image.authors="UtsavBalar1231 <utsavbalar1231@gmail.com>"
