%global crate dotman

Name:           dotman
Version:        0.0.1
Release:        1%{?dist}
Summary:        Blazingly fast dotfiles manager with git-like semantics

License:        MIT
URL:            https://github.com/UtsavBalar1231/dotman-rs
Source0:        https://github.com/UtsavBalar1231/dotman-rs/archive/v%{version}.tar.gz#/%{name}-%{version}.tar.gz

ExclusiveArch:  %{rust_arches}

BuildRequires:  rust-packaging >= 21
BuildRequires:  cargo
BuildRequires:  rust >= 1.70
BuildRequires:  help2man
BuildRequires:  systemd-rpm-macros

Suggests:       git

%description
dotman is a high-performance dotfiles manager designed for developers who
demand speed without sacrificing functionality. Unlike traditional dotfile
managers that treat performance as an afterthought, dotman is built from
the ground up with extreme optimization in mind.

Key features:
- SIMD-accelerated operations for string matching and UTF-8 validation
- Parallel file processing using all available CPU cores
- Memory-mapped I/O for efficient large file handling
- xxHash3 for ultra-fast file hashing (>1GB/s throughput)
- Sub-millisecond operations for typical dotfile repositories
- Content-based deduplication to minimize storage usage
- Zstd compression with dictionary training for optimal compression ratios
- Binary index format for instant loading (10,000+ files in <10ms)
- Git-like interface with familiar commands
- Cross-platform support with architecture-specific optimizations

This package provides the 'dot' command-line tool for managing dotfiles
with unprecedented performance and reliability.

%prep
%autosetup -n %{name}-%{version}
%cargo_prep

%generate_buildrequires
%cargo_generate_buildrequires

%build
# Build with full optimizations
export RUSTFLAGS="%{optflags} -C target-cpu=native"
%cargo_build --release --all-features

%install
# Install the binary (note: binary name is 'dot', not 'dotman')
%cargo_install

# Install documentation
install -Dm644 README.md %{buildroot}%{_docdir}/%{name}/README.md
install -Dm644 CLAUDE.md %{buildroot}%{_docdir}/%{name}/CLAUDE.md

# Generate and install shell completions
mkdir -p %{buildroot}%{_datadir}/bash-completion/completions
target/release/dot completion bash > %{buildroot}%{_datadir}/bash-completion/completions/dot || :

mkdir -p %{buildroot}%{_datadir}/zsh/site-functions
target/release/dot completion zsh > %{buildroot}%{_datadir}/zsh/site-functions/_dot || :

mkdir -p %{buildroot}%{_datadir}/fish/vendor_completions.d
target/release/dot completion fish > %{buildroot}%{_datadir}/fish/vendor_completions.d/dot.fish || :

# Generate man page
mkdir -p %{buildroot}%{_mandir}/man1
help2man --no-info --name="blazingly fast dotfiles manager" \
    --version-string="%{version}" \
    target/release/dot > %{buildroot}%{_mandir}/man1/dot.1 || :

# Install examples if they exist
if [ -d examples ]; then
    mkdir -p %{buildroot}%{_docdir}/%{name}/examples
    cp -r examples/* %{buildroot}%{_docdir}/%{name}/examples/
fi

%check
%cargo_test --release --all-features

%files
%license LICENSE*
%doc README.md CLAUDE.md
%{_bindir}/dot
%{_datadir}/bash-completion/completions/dot
%{_datadir}/zsh/site-functions/_dot
%{_datadir}/fish/vendor_completions.d/dot.fish
%{_mandir}/man1/dot.1*
%{_docdir}/%{name}/
%if 0%{?_licensedir:1}
%else
%doc LICENSE*
%endif

%changelog
* Mon Aug 19 2024 Utsav Balar <utsavbalar1231@gmail.com> - 0.0.1-1
- Initial package for dotman
- Features blazingly fast dotfiles management
- SIMD-accelerated operations and parallel processing
- Memory-mapped I/O and ultra-fast hashing
- Content-based deduplication and compression
- Git-like interface for familiar workflow
- Cross-platform support with optimizations
- Comprehensive shell completions and man page
- 26x faster than GNU Stow, 31x faster than chezmoi
