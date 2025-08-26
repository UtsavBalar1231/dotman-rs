%global crate dotman

Name:           dotman
Version:        0.0.1
Release:        1%{?dist}
Summary:        Blazingly fast dotfiles manager with git-like semantics

License:        MIT
URL:            https://github.com/UtsavBalar1231/dotman-rs
Source0:        dotman-%{version}.tar.gz

BuildRequires:  cargo
BuildRequires:  rust >= 1.70

# Disable debuginfo on Arch (not supported)
%global debug_package %{nil}

%description
dotman is a high-performance dotfiles manager designed for developers who
demand speed without sacrificing functionality.

%prep
%setup -q

%build
export RUSTFLAGS="-C target-cpu=native"
cargo build --release --all-features

%install
install -D -m 755 target/release/dot %{buildroot}%{_bindir}/dot
install -D -m 644 README.md %{buildroot}%{_docdir}/%{name}/README.md

%files
%{_bindir}/dot
%doc %{_docdir}/%{name}/

%changelog
* Mon Aug 19 2024 Utsav Balar <utsavbalar1231@gmail.com> - 0.0.1-1
- Initial package for dotman
