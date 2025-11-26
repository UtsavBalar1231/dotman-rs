Name:           dotman
Version:        0.0.1
Release:        1%{?dist}
Summary:        High-performance dotfiles manager with git-like semantics

License:        MIT
URL:            https://github.com/UtsavBalar1231/dotman
Source0:        https://github.com/UtsavBalar1231/dotman/archive/v%{version}.tar.gz#/%{name}-%{version}.tar.gz

BuildRequires:  cargo >= 1.70
BuildRequires:  rust >= 1.70
BuildRequires:  help2man
BuildRequires:  gcc

Suggests:       git

%description
dotman is a high-performance dotfiles manager designed for developers who
demand speed without sacrificing functionality. Built in Rust with SIMD
acceleration, parallel processing, and content deduplication.

Key features:
- SIMD-accelerated operations for maximum performance
- Parallel file processing using all available CPU cores
- Memory-mapped I/O for efficient large file handling
- xxHash3 for ultra-fast file hashing (>1GB/s throughput)
- Content-based deduplication and Zstd compression
- Git-like interface with familiar commands

This package provides the 'dot' command-line tool.

%prep
%autosetup -n %{name}-%{version}

%build
cargo build --release --locked --all-features

%install
# Install binary
install -Dm755 target/release/dot %{buildroot}%{_bindir}/dot

# Install documentation
install -Dm644 README.md %{buildroot}%{_docdir}/%{name}/README.md

# Generate and install shell completions
mkdir -p %{buildroot}%{_datadir}/bash-completion/completions
target/release/dot completion bash > %{buildroot}%{_datadir}/bash-completion/completions/dot

mkdir -p %{buildroot}%{_datadir}/zsh/site-functions
target/release/dot completion zsh > %{buildroot}%{_datadir}/zsh/site-functions/_dot

mkdir -p %{buildroot}%{_datadir}/fish/vendor_completions.d
target/release/dot completion fish > %{buildroot}%{_datadir}/fish/vendor_completions.d/dot.fish

# Generate and install man page
mkdir -p %{buildroot}%{_mandir}/man1
help2man --no-info --name="high-performance dotfiles manager" \
    --version-string="%{version}" target/release/dot > %{buildroot}%{_mandir}/man1/dot.1

%check
cargo test --release --locked --all-features

%files
%license LICENSE
%doc README.md
%{_bindir}/dot
%{_datadir}/bash-completion/completions/dot
%{_datadir}/zsh/site-functions/_dot
%{_datadir}/fish/vendor_completions.d/dot.fish
%{_mandir}/man1/dot.1*
%{_docdir}/%{name}/

%changelog
* Tue Nov 26 2024 Utsav Balar <utsavbalar1231@gmail.com> - 0.0.1-1
- Initial package
- High-performance dotfiles management with git-like semantics
- SIMD acceleration and parallel processing
- Content-based deduplication and compression
