# Security Policy

## Supported Versions

dotman is currently in pre-release development (version 0.0.1). We actively support the current development version and provide security updates as needed.

### Platform Support

**Tier 1 Platforms** (Binary releases provided):
- Linux x86_64 (glibc-based distributions: Debian, Ubuntu, Fedora, etc.)
- Linux x86_64 (musl-based distributions: Alpine Linux)
- Linux aarch64 (glibc-based distributions)
- Linux aarch64 (musl-based distributions)

**Tier 2 Platforms** (Source builds supported):
- macOS (x86_64 and aarch64)
- Windows (x86_64)

| Version | Supported          | Status                    |
| ------- | ------------------ | ------------------------- |
| 0.0.1   | :white_check_mark: | Pre-release, active dev   |

## Reporting a Vulnerability

We take security vulnerabilities seriously and appreciate responsible disclosure from the security community.

### How to Report

**Preferred Method: GitHub Security Advisories**

Use [GitHub's private vulnerability reporting](https://github.com/utsavbalar1231/dotman/security/advisories/new) for secure, private disclosure. This is the recommended approach as it provides:
- Private communication channel
- Coordinated disclosure workflow
- Integration with GitHub's security features

**Alternative Method: Email**

If you cannot use GitHub Security Advisories, please email security reports to:
- **Email**: utsavbalar1231@gmail.com
- **Subject**: [SECURITY] Brief description of the vulnerability

### What to Include

Please provide the following information in your report:

1. **Description**: Clear explanation of the vulnerability
2. **Reproduction Steps**: Detailed steps to reproduce the issue
3. **Impact Assessment**: Your analysis of the potential security impact
4. **Affected Versions**: Versions where the vulnerability exists
5. **Proof of Concept**: Code or commands demonstrating the issue (if applicable)
6. **Suggested Fix**: Any recommendations for fixing the vulnerability (optional)

### Response Timeline

- **Acknowledgment**: Within 48 hours of receiving your report
- **Initial Assessment**: Within 7 days with our evaluation of severity
- **Fix Timeline**: Within 90 days for high/critical severity issues
- **Public Disclosure**: Coordinated with the reporter after fix is released

## Security Measures

dotman implements multiple security measures to maintain code and dependency security:

### Automated Security Scanning

- **cargo-audit**: Automated scanning for known vulnerabilities in dependencies
- **cargo-deny**: License compliance and security advisory checking
- **CI Integration**: Security checks run on every pull request and merge
- **Static Analysis**: Clippy with security-focused lints enabled

### Dependency Management

- **Dependabot**: Weekly automated dependency updates
- **Minimal Dependencies**: Careful evaluation of third-party crates
- **Dependency Pinning**: Lock file ensures reproducible builds
- **Regular Audits**: Periodic manual review of dependency tree

### Security-Focused Testing

- **Comprehensive Test Suite**: Unit, integration, and property-based tests
- **Security Test Cases**: Tests specifically targeting security concerns
- **Fuzzing**: Property-based testing with proptest for edge cases
- **Continuous Integration**: All tests run on multiple platforms

## Known Security Considerations

The following areas require careful attention and are documented for transparency:

### 1. Path Validation (Path Traversal Prevention)

**Location**: `src/commands/add.rs` and path handling throughout codebase

**Concern**: Potential path traversal vulnerabilities with relative paths like `../../.ssh/id_rsa`

**Mitigation Status**: Under development
- Need path canonicalization before processing
- Validation against allowed directories (home directory scope)
- Rejection of paths outside permitted boundaries

**User Impact**: Users should avoid using relative paths with `..` components when adding files

### 2. Permission Handling (Setuid/Setgid/Sticky Bits)

**Location**: `src/storage/index.rs`, `src/utils/permissions.rs`

**Concern**: Setuid, setgid, and sticky bits are currently preserved in file metadata

**Mitigation Status**: Under review
- Plan to strip dangerous permission bits (0o4000, 0o2000, 0o1000) during storage
- Preserve only standard rwx permissions (0o777)
- Add explicit permission validation before file restoration

**User Impact**: Exercise caution when tracking files with special permission bits

### 3. Repository Access Validation

**Location**: All write operations throughout codebase

**Concern**: Insufficient permission validation before write operations

**Mitigation Status**: Under development
- Early permission checks before repository modifications
- Directory and file write permission validation
- Use of `fs4` lock system to prevent race conditions

**User Impact**: Ensure proper permissions on `~/.dotman` directory (recommended: 0700)

## Disclosure Policy

We follow a **90-day coordinated disclosure policy**:

1. **Private Disclosure**: Report received and acknowledged privately
2. **Fix Development**: Security fix developed and tested
3. **Pre-Release Testing**: Fix validated across supported platforms
4. **Coordinated Release**: Public release coordinated with reporter
5. **Public Advisory**: Security advisory published after fix is available
6. **Credit**: Reporters credited in release notes and security advisory (unless anonymity requested)

### Disclosure Timeline

- **Day 0**: Vulnerability reported
- **Day 1-2**: Acknowledgment sent to reporter
- **Day 1-7**: Initial assessment and severity determination
- **Day 7-90**: Fix development, testing, and validation
- **Day 90**: Public disclosure and release (or earlier if agreed upon)

## Security Best Practices for Users

### Safe Usage Guidelines

1. **Avoid Tracking Sensitive Files**
   - Never track SSH keys (`~/.ssh/id_rsa`, `~/.ssh/id_ed25519`)
   - Exclude API keys and authentication tokens
   - Don't track files containing passwords or credentials
   - Review files before adding them to dotman

2. **Secure Repository Permissions**
   - Set appropriate permissions on `~/.dotman` directory:
     ```bash
     chmod 700 ~/.dotman
     ```
   - Restrict access to your user account only
   - Avoid running dotman with elevated privileges unless necessary

3. **Verify Binary Integrity**
   - Download releases from official GitHub releases only
   - Verify SHA256 checksums provided with releases:
     ```bash
     sha256sum -c checksums.txt
     ```
   - Consider verifying GPG signatures when available

4. **Configuration Security**
   - Review configuration file permissions (`~/.config/dotman/config`)
   - Use environment variables for sensitive configuration (not config file)
   - Regularly audit tracked files with `dot status`

5. **Remote Repository Security**
   - Use SSH keys for Git remote operations (not HTTPS passwords)
   - Enable two-factor authentication on remote Git services
   - Regularly rotate SSH keys used for remote access
   - Review remote repository access logs periodically

6. **Regular Updates**
   - Keep dotman updated to the latest version
   - Monitor release notes for security fixes
   - Subscribe to security advisories on GitHub

### Ignore Patterns for Sensitive Data

Create or update `~/.dotman/ignore` to exclude sensitive patterns:

```
# SSH keys
.ssh/id_*
.ssh/*.pem

# GPG keys
.gnupg/private-keys-v1.d/*
.gnupg/secring.gpg

# Authentication tokens
.aws/credentials
.docker/config.json
.netrc

# API keys
*.key
*.pem
*_key
*_secret

# Password files
.password-store/*
.pass/*

# Browser data
.mozilla/firefox/*/key*.db
.config/google-chrome/Default/Login Data
```

## Security Audit Information

For security researchers and auditors:

### Code Organization

- **Entry Point**: `src/main.rs` - CLI interface
- **Core Context**: `src/lib.rs` - Central repository context
- **Command Modules**: `src/commands/` - Individual command implementations
- **Storage Layer**: `src/storage/` - File storage and indexing
- **Security-Critical Paths**:
  - Path handling: `src/commands/add.rs`, `src/utils/paths.rs`
  - Permission handling: `src/utils/permissions.rs`, `src/storage/index.rs`
  - Repository operations: All `src/commands/*.rs` files

### Build and Test

```bash
# Clone repository
git clone https://github.com/utsavbalar1231/dotman.git
cd dotman

# Run security-focused tests
cargo test --test comprehensive_unit_tests

# Run all tests
cargo test

# Security audit dependencies
cargo audit

# Check for security lints
cargo clippy -- -D warnings

# Build for security testing
cargo build --release
```

### Known Safe Practices

- **No Unsafe Code**: Codebase avoids `unsafe` blocks where possible
- **Memory Safety**: Rust's memory safety guarantees prevent common vulnerabilities
- **Error Handling**: Uses `anyhow` for comprehensive error context
- **Dependency Auditing**: Regular automated and manual dependency reviews
- **Input Validation**: User input validation at command boundaries

## Contact

- **Security Issues**: Use GitHub Security Advisories or email utsavbalar1231@gmail.com
- **General Issues**: Use [GitHub Issues](https://github.com/utsavbalar1231/dotman/issues)
- **Questions**: Use [GitHub Discussions](https://github.com/utsavbalar1231/dotman/discussions)

## Acknowledgments

We appreciate the security research community's efforts in responsible disclosure. Security researchers who report valid vulnerabilities will be acknowledged in:

- Release notes for the fixing version
- Security advisories published on GitHub
- This SECURITY.md file (Hall of Fame section, when applicable)

Thank you for helping keep dotman secure!

---

**Last Updated**: 2025-11-25
**Version**: 0.0.1-pre
