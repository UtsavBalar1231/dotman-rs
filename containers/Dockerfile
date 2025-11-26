# Multi-stage Dockerfile for dotman
# Builds an optimized release binary in a Debian-based container
#
# Build: docker build -f containers/Dockerfile -t dotman:latest .
# Run:   docker run --rm dotman:latest --version

FROM rust:bookworm AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    help2man \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

# Create build directory
WORKDIR /usr/src/app

# Copy source
COPY . .

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

ENTRYPOINT ["/usr/local/bin/dot"]
CMD ["--help"]

# Metadata
LABEL org.opencontainers.image.title="dotman"
LABEL org.opencontainers.image.description="High-performance dotfiles manager"
LABEL org.opencontainers.image.url="https://github.com/UtsavBalar1231/dotman"
LABEL org.opencontainers.image.source="https://github.com/UtsavBalar1231/dotman"
LABEL org.opencontainers.image.licenses="MIT"
LABEL org.opencontainers.image.authors="UtsavBalar1231 <utsavbalar1231@gmail.com>"
