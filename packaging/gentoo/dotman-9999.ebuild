# Copyright 2024 Gentoo Authors
# Distributed under the terms of the GNU General Public License v2

EAPI=8

inherit cargo git-r3

DESCRIPTION="Blazingly fast dotfiles manager with git-like semantics (live version)"
HOMEPAGE="https://github.com/UtsavBalar1231/dotman-rs"
EGIT_REPO_URI="https://github.com/UtsavBalar1231/dotman-rs.git"

LICENSE="MIT"
# Additional licenses for dependencies will be determined at build time
LICENSE+=" Apache-2.0 Apache-2.0-with-LLVM-exceptions BSD ISC MIT MPL-2.0 Unicode-DFS-2016 Unlicense ZLIB"

SLOT="0"
KEYWORDS=""
IUSE="doc"

# Build dependencies
BDEPEND="
	>=virtual/rust-1.70.0
	app-text/help2man
	net-misc/curl
"

# Runtime dependencies (none for static binary)
RDEPEND=""

# Test dependencies
DEPEND="
	${RDEPEND}
"

QA_FLAGS_IGNORED="usr/bin/dot"

src_unpack() {
	git-r3_src_unpack
	cargo_live_src_unpack
}

src_configure() {
	# Set optimization flags
	export RUSTFLAGS="-C target-cpu=native -C opt-level=3 -C lto=fat -C strip=symbols"
	
	# Configure cargo
	cargo_src_configure
}

src_compile() {
	# Build with all features enabled
	cargo_src_compile --bin dot --all-features
}

src_test() {
	# Run test suite
	cargo_src_test --all-features
}

src_install() {
	# Install the binary
	cargo_src_install --bin dot
	
	# Install documentation
	dodoc README.md CLAUDE.md
	
	# Generate and install shell completions
	local completions_dir="${T}/completions"
	mkdir -p "${completions_dir}" || die
	
	"${S}/target/release/dot" completion bash > "${completions_dir}/dot" || die
	"${S}/target/release/dot" completion zsh > "${completions_dir}/_dot" || die
	"${S}/target/release/dot" completion fish > "${completions_dir}/dot.fish" || die
	
	# Install completions
	insinto /usr/share/bash-completion/completions
	doins "${completions_dir}/dot"
	
	insinto /usr/share/zsh/site-functions
	doins "${completions_dir}/_dot"
	
	insinto /usr/share/fish/vendor_completions.d
	doins "${completions_dir}/dot.fish"
	
	# Generate and install man page
	local version
	version=$("${S}/target/release/dot" --version | cut -d' ' -f2)
	help2man --no-info --name="blazingly fast dotfiles manager" \
		--version-string="${version}" "${S}/target/release/dot" > "${T}/dot.1" || die
	doman "${T}/dot.1"
	
	# Install additional documentation if requested
	if use doc; then
		cargo_src_install --doc
	fi
}

pkg_postinst() {
	elog "dotman live version has been successfully installed!"
	elog ""
	elog "This is the development version that tracks the main branch."
	elog "It may contain unstable features and breaking changes."
	elog ""
	elog "To get started:"
	elog "  dot init          # Initialize a new dotfiles repository"
	elog "  dot --help        # Show available commands"
	elog ""
	elog "Shell completions have been installed for bash, zsh, and fish."
	elog "You may need to restart your shell or source the completion files."
	elog ""
	elog "For more information, see: https://github.com/UtsavBalar1231/dotman-rs"
}