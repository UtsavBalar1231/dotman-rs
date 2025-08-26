# Copyright 2024 Gentoo Authors
# Distributed under the terms of the GNU General Public License v2

EAPI=8

CRATES="
	# This will be populated with the actual crate dependencies
	# when the package is submitted to the Gentoo repository
	# Generate with: cargo generate-lockfile && cargo metadata --format-version 1 | jq -r '.packages[] | select(.source != null) | "\(.name)-\(.version)"' | sort
"

inherit cargo

DESCRIPTION="Blazingly fast dotfiles manager with git-like semantics"
HOMEPAGE="https://github.com/UtsavBalar1231/dotman-rs"
SRC_URI="
	https://github.com/UtsavBalar1231/dotman-rs/archive/refs/tags/v${PV}.tar.gz -> ${P}.tar.gz
	$(cargo_crate_uris ${CRATES})
"

LICENSE="MIT"
# Additional licenses for dependencies will be added here
LICENSE+=" Apache-2.0 Apache-2.0-with-LLVM-exceptions BSD ISC MIT MPL-2.0 Unicode-DFS-2016 Unlicense ZLIB"

SLOT="0"
KEYWORDS="~amd64 ~arm64"
IUSE="doc"

# Build dependencies
BDEPEND="
	>=virtual/rust-1.70.0
	app-text/help2man
"

# Runtime dependencies (none for static binary)
RDEPEND=""

# Test dependencies
DEPEND="
	${RDEPEND}
"

# Use the new EAPI 8 src_unpack for cargo
QA_FLAGS_IGNORED="usr/bin/dot"

src_configure() {
	# Set optimization flags
	export RUSTFLAGS="-C target-cpu=native -C opt-level=3 -C lto=fat -C strip=symbols"
	
	# Configure cargo
	cargo_src_configure --offline
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
	dodoc README.md
	
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
	help2man --no-info --name="blazingly fast dotfiles manager" \
		--version-string="${PV}" "${S}/target/release/dot" > "${T}/dot.1" || die
	doman "${T}/dot.1"
	
	# Install additional documentation if requested
	if use doc; then
		cargo_src_install --doc
	fi
}

pkg_postinst() {
	elog "dotman has been successfully installed!"
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
