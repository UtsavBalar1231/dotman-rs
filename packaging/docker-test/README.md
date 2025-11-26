# Docker-based Package Testing

This directory contains Dockerfiles for testing dotman package builds across different Linux distributions.

## Overview

Each Dockerfile performs a complete package build and installation test:
1. Installs build dependencies
2. Builds dotman from source
3. Creates a distribution package
4. Installs the package in a clean environment
5. Runs smoke tests to verify the installation

## Available Formats

| Format | Dockerfile | Description |
|--------|------------|-------------|
| Debian | `debian.Dockerfile` | Builds `.deb` package for Debian/Ubuntu |
| Arch | `arch.Dockerfile` | Builds `.pkg.tar.zst` package for Arch Linux |
| Alpine | `alpine.Dockerfile` | Builds static musl binary for Alpine |
| Fedora | `fedora.Dockerfile` | Builds `.rpm` package for Fedora/RHEL/CentOS |

## Usage

### Using the build script

```bash
# Build all formats
./build-all.sh

# Build specific format
./build-all.sh debian
./build-all.sh arch alpine

# Build without cache
./build-all.sh --no-cache debian

# Build all in parallel
./build-all.sh --parallel
```

### Using Docker directly

```bash
# From project root directory
cd /path/to/dotman

# Build Debian package
docker build -f packaging/docker-test/debian.Dockerfile -t dotman-test-deb .

# Build Arch package
docker build -f packaging/docker-test/arch.Dockerfile -t dotman-test-arch .

# Build Alpine package
docker build -f packaging/docker-test/alpine.Dockerfile -t dotman-test-alpine .

# Build RPM package
docker build -f packaging/docker-test/fedora.Dockerfile -t dotman-test-rpm .
```

### Verify Installation

```bash
# Run the built container
docker run --rm dotman-test-deb dot --version

# Interactive shell for debugging
docker run --rm -it dotman-test-deb /bin/bash
```

## What Gets Tested

Each build verifies:

- [x] Binary compiles successfully
- [x] Package builds without errors
- [x] Package installs cleanly
- [x] `dot --version` works
- [x] `dot --help` works
- [x] `dot init` works
- [x] `dot status` works
- [x] Shell completions are installed (bash, zsh, fish)
- [x] Man page is installed

## Smoke Tests

The following commands are run in each container:

```bash
dot --version
dot --help
dot init
dot status
```

## CI Integration

These Dockerfiles are used in CI to ensure packages build correctly:

```yaml
# .github/workflows/ci.yml
package-test:
  runs-on: ubuntu-latest
  strategy:
    matrix:
      package: [debian, arch, alpine, fedora]
  steps:
    - uses: actions/checkout@v4
    - name: Build package
      run: |
        docker build \
          -f packaging/docker-test/${{ matrix.package }}.Dockerfile \
          -t dotman-${{ matrix.package }}-test .
    - name: Verify installation
      run: docker run --rm dotman-${{ matrix.package }}-test dot --version
```

## Troubleshooting

### Build fails with "permission denied"

Make sure Docker has access to the project directory:
```bash
docker build --network=host -f packaging/docker-test/debian.Dockerfile -t dotman-test-deb .
```

### Arch build fails with "makepkg cannot be run as root"

This is expected - the Dockerfile creates a non-root user for the build.

### Alpine build produces non-static binary

Check that RUSTFLAGS includes `-C target-feature=+crt-static`:
```bash
docker run --rm dotman-test-alpine file /usr/bin/dot
```

### RPM build fails with missing dependencies

The Fedora Dockerfile uses a simplified spec file that doesn't require `rust-packaging`.
If you need the full spec, install `rust-packaging >= 21`.
