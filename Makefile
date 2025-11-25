# Makefile for dotman - Blazingly fast dotfiles manager
# Traditional GNU Make build system for compatibility
# For modern task automation, see Justfile

# Project Configuration
PROJECT_NAME := dotman
BINARY_NAME := dot
VERSION := $(shell grep '^version = ' Cargo.toml | sed 's/.*"\(.*\)".*/\1/')
RUST_VERSION := $(shell grep '^rust-version = ' Cargo.toml | sed 's/.*"\(.*\)".*/\1/' || echo "1.70.0")

# Build Configuration
CARGO := cargo
CARGO_FLAGS := --release --all-features
RUSTFLAGS := -C target-cpu=native

# Installation Paths
PREFIX ?= /usr/local
BINDIR := $(PREFIX)/bin
MANDIR := $(PREFIX)/share/man/man1
DATADIR := $(PREFIX)/share/$(PROJECT_NAME)
COMPLETIONDIR := $(PREFIX)/share/bash-completion/completions

# Platform Detection
UNAME_S := $(shell uname -s)
UNAME_M := $(shell uname -m)

# Platform-specific settings
ifeq ($(UNAME_S),Linux)
    PLATFORM := linux
    TARGET_SUFFIX :=
endif
ifeq ($(UNAME_S),Darwin)
    PLATFORM := macos
    TARGET_SUFFIX :=
    # macOS specific flags
    RUSTFLAGS += -C link-arg=-Wl,-dead_strip
endif
ifeq ($(UNAME_S),Windows_NT)
    PLATFORM := windows
    TARGET_SUFFIX := .exe
    BINARY_NAME := $(BINARY_NAME)$(TARGET_SUFFIX)
endif

# Architecture Detection
ifeq ($(UNAME_M),x86_64)
    ARCH := x86_64
endif
ifeq ($(UNAME_M),aarch64)
    ARCH := aarch64
    RUSTFLAGS := -C target-cpu=generic -C opt-level=3 -C lto=fat
endif
ifeq ($(UNAME_M),arm64)
    ARCH := aarch64
    RUSTFLAGS := -C target-cpu=generic -C opt-level=3 -C lto=fat
endif

# Build directory
BUILD_DIR := target/release
BINARY_PATH := $(BUILD_DIR)/$(BINARY_NAME)

# Default target
.DEFAULT_GOAL := all
.PHONY: all build install uninstall clean test check lint fmt docs help

# Main targets
all: build

# Build the project
build:
	@echo "Building $(PROJECT_NAME) v$(VERSION) for $(PLATFORM)-$(ARCH)"
	RUSTFLAGS="$(RUSTFLAGS)" $(CARGO) build $(CARGO_FLAGS)
	@echo "Build complete: $(BINARY_PATH)"

# Build with debug information
build-debug:
	@echo "Building $(PROJECT_NAME) v$(VERSION) with debug symbols"
	$(CARGO) build --profile release-with-debug --all-features
	@echo "Debug build complete: target/release-with-debug/$(BINARY_NAME)"

# Run tests
test:
	@echo "Running test suite..."
	$(CARGO) test --all-features
	@echo "All tests passed!"

# Comprehensive check (lint + test + security)
check: lint test security
	@echo "All checks passed!"

# Code linting
lint:
	@echo "Running clippy linter..."
	$(CARGO) clippy --all-targets --all-features -- -D warnings
	@echo "Linting complete!"

# Code formatting
fmt:
	@echo "Formatting code..."
	$(CARGO) fmt --all
	@echo "Formatting complete!"

# Format check
fmt-check:
	@echo "Checking code formatting..."
	$(CARGO) fmt --all -- --check

# Security audit
security:
	@echo "Running security audit..."
	@if command -v cargo-audit >/dev/null 2>&1; then \
		$(CARGO) audit; \
	else \
		echo "cargo-audit not found. Install with: cargo install cargo-audit"; \
		exit 1; \
	fi
	@echo "Security audit complete!"

# Generate documentation
docs:
	@echo "Generating documentation..."
	$(CARGO) doc --no-deps --all-features --open
	@echo "Documentation generated!"

# Benchmark
bench:
	@echo "Running benchmarks..."
	$(CARGO) bench
	@echo "Benchmarks complete!"

# Install the binary and supporting files
install: build install-binary install-completions install-man
	@echo "Installation complete!"
	@echo "Run '$(BINARY_NAME) --help' to get started"

# Install only the binary
install-binary: build
	@echo "Installing binary to $(BINDIR)/$(BINARY_NAME)"
	@mkdir -p $(BINDIR)
	install -m 755 $(BINARY_PATH) $(BINDIR)/$(BINARY_NAME)

# Install shell completions
install-completions: build
	@echo "Installing shell completions..."
	@mkdir -p $(PREFIX)/share/bash-completion/completions
	@mkdir -p $(PREFIX)/share/zsh/site-functions
	@mkdir -p $(PREFIX)/share/fish/vendor_completions.d
	$(BINARY_PATH) completion bash > $(PREFIX)/share/bash-completion/completions/$(BINARY_NAME)
	$(BINARY_PATH) completion zsh > $(PREFIX)/share/zsh/site-functions/_$(BINARY_NAME)
	$(BINARY_PATH) completion fish > $(PREFIX)/share/fish/vendor_completions.d/$(BINARY_NAME).fish

# Install man page
install-man: build
	@echo "Installing man page..."
	@mkdir -p $(MANDIR)
	@if command -v help2man >/dev/null 2>&1; then \
		help2man --no-info --name="blazingly fast dotfiles manager" \
			--version-string="$(VERSION)" \
			$(BINARY_PATH) > $(MANDIR)/$(BINARY_NAME).1; \
	else \
		echo "help2man not found. Skipping man page generation."; \
		echo "Install help2man to generate man pages during installation."; \
	fi

# Uninstall
uninstall:
	@echo "Uninstalling $(PROJECT_NAME)..."
	rm -f $(BINDIR)/$(BINARY_NAME)
	rm -f $(MANDIR)/$(BINARY_NAME).1
	rm -f $(PREFIX)/share/bash-completion/completions/$(BINARY_NAME)
	rm -f $(PREFIX)/share/zsh/site-functions/_$(BINARY_NAME)
	rm -f $(PREFIX)/share/fish/vendor_completions.d/$(BINARY_NAME).fish
	@echo "Uninstallation complete!"

# Clean build artifacts
clean:
	@echo "Cleaning build artifacts..."
	$(CARGO) clean
	rm -rf dist/
	@echo "Clean complete!"

# Development targets
dev: build test lint
	@echo "Development build complete!"

# CI pipeline
ci: fmt-check lint test security
	@echo "CI pipeline complete!"

# Release preparation
release-prepare: clean ci
	@echo "Preparing release $(VERSION)..."
	@echo "✓ Code formatted and linted"
	@echo "✓ Tests passing"
	@echo "✓ Security audit clean"
	@echo "Ready for release!"

# Cross-compilation targets
build-linux:
	@echo "Building for Linux targets..."
	$(CARGO) build --release --target x86_64-unknown-linux-gnu
	$(CARGO) build --release --target aarch64-unknown-linux-gnu
	$(CARGO) build --release --target x86_64-unknown-linux-musl
	$(CARGO) build --release --target aarch64-unknown-linux-musl

build-macos:
	@echo "Building for macOS targets..."
	$(CARGO) build --release --target x86_64-apple-darwin
	$(CARGO) build --release --target aarch64-apple-darwin

build-windows:
	@echo "Building for Windows targets..."
	$(CARGO) build --release --target x86_64-pc-windows-gnu

# Build for all supported platforms
build-all: build-linux build-macos build-windows
	@echo "Cross-compilation complete!"

# Package creation
package-linux: build-linux
	@echo "Creating Linux packages..."
	@mkdir -p dist
	tar czf dist/$(PROJECT_NAME)-$(VERSION)-linux-x86_64.tar.gz \
		-C target/x86_64-unknown-linux-gnu/release $(BINARY_NAME)
	tar czf dist/$(PROJECT_NAME)-$(VERSION)-linux-aarch64.tar.gz \
		-C target/aarch64-unknown-linux-gnu/release $(BINARY_NAME)

package-macos: build-macos
	@echo "Creating macOS packages..."
	@mkdir -p dist
	tar czf dist/$(PROJECT_NAME)-$(VERSION)-macos-x86_64.tar.gz \
		-C target/x86_64-apple-darwin/release $(BINARY_NAME)
	tar czf dist/$(PROJECT_NAME)-$(VERSION)-macos-aarch64.tar.gz \
		-C target/aarch64-apple-darwin/release $(BINARY_NAME)

package-windows: build-windows
	@echo "Creating Windows packages..."
	@mkdir -p dist
	cd target/x86_64-pc-windows-gnu/release && \
		zip -r ../../../dist/$(PROJECT_NAME)-$(VERSION)-windows-x86_64.zip $(BINARY_NAME)

# Create all distribution packages
package-all: package-linux package-macos package-windows
	@echo "All packages created in dist/"

# Docker targets
docker-build:
	@echo "Building Docker image..."
	docker build -f containers/Dockerfile -t $(PROJECT_NAME):latest .

docker-build-alpine:
	@echo "Building Alpine Docker image..."
	docker build -f containers/Dockerfile.alpine -t $(PROJECT_NAME):alpine .

# Version information
version:
	@echo "$(PROJECT_NAME) v$(VERSION)"
	@echo "Rust version: $(RUST_VERSION)"
	@echo "Platform: $(PLATFORM)-$(ARCH)"

# Help target
help:
	@echo "$(PROJECT_NAME) v$(VERSION) - Build System"
	@echo ""
	@echo "USAGE:"
	@echo "    make [target]"
	@echo ""
	@echo "TARGETS:"
	@echo "    all                Build the project (default)"
	@echo "    build              Build optimized binary"
	@echo "    build-debug        Build with debug symbols"
	@echo "    test               Run test suite"
	@echo "    check              Run lint + test + security"
	@echo "    lint               Run clippy linter"
	@echo "    fmt                Format source code"
	@echo "    fmt-check          Check code formatting"
	@echo "    security           Run security audit"
	@echo "    docs               Generate documentation"
	@echo "    bench              Run benchmarks"
	@echo "    install            Install binary and support files"
	@echo "    install-binary     Install only the binary"
	@echo "    uninstall          Remove installed files"
	@echo "    clean              Remove build artifacts"
	@echo ""
	@echo "CROSS-COMPILATION:"
	@echo "    build-linux        Build for all Linux targets"
	@echo "    build-macos        Build for all macOS targets"
	@echo "    build-windows      Build for Windows targets"
	@echo "    build-all          Build for all platforms"
	@echo ""
	@echo "PACKAGING:"
	@echo "    package-linux      Create Linux distribution packages"
	@echo "    package-macos      Create macOS distribution packages"
	@echo "    package-windows    Create Windows distribution packages"
	@echo "    package-all        Create all distribution packages"
	@echo ""
	@echo "DOCKER:"
	@echo "    docker-build       Build Docker image (Debian-based)"
	@echo "    docker-build-alpine Build Alpine Docker image"
	@echo ""
	@echo "DEVELOPMENT:"
	@echo "    dev                Build + test + lint"
	@echo "    ci                 Full CI pipeline"
	@echo "    release-prepare    Prepare for release"
	@echo "    version            Show version information"
	@echo "    help               Show this help message"
	@echo ""
	@echo "VARIABLES:"
	@echo "    PREFIX=$(PREFIX)    Installation prefix"
	@echo "    CARGO_FLAGS=$(CARGO_FLAGS)  Cargo build flags"
	@echo "    RUSTFLAGS=$(RUSTFLAGS)"
	@echo ""
	@echo "For modern task automation, see: just --list"
