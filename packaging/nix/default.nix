# Traditional Nix expression for dotman
# Compatible with older Nix versions that don't support flakes
#
# Usage:
#   nix-build
#   nix-shell  # for development environment
#
# Or with nixpkgs:
#   nix-build -E 'with import <nixpkgs> {}; callPackage ./default.nix {}'

{ lib
, stdenv
, fetchFromGitHub
, rustPlatform
, pkg-config
, installShellFiles
, help2man
, darwin
}:

rustPlatform.buildRustPackage rec {
  pname = "dotman";
  version = "0.0.1";

  src = fetchFromGitHub {
    owner = "UtsavBalar1231";
    repo = "dotman";
    rev = "v${version}";
    hash = "sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=";  # Update with: nix-prefetch-github UtsavBalar1231 dotman --rev v${version}
  };

  cargoHash = "sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=";  # Update after first build attempt

  nativeBuildInputs = [
    pkg-config
    installShellFiles
    help2man
  ];

  buildInputs = lib.optionals stdenv.isDarwin (with darwin.apple_sdk.frameworks; [
    Security
    SystemConfiguration
  ]);

  # Build with all features
  buildFeatures = [ "default" ];

  # Run tests during build
  doCheck = true;

  # Post-install: add shell completions and man page
  postInstall = ''
    # Generate shell completions
    installShellCompletion --cmd dot \
      --bash <($out/bin/dot completion bash) \
      --zsh <($out/bin/dot completion zsh) \
      --fish <($out/bin/dot completion fish)

    # Generate man page
    help2man --no-info --name="high-performance dotfiles manager" \
      --version-string="${version}" \
      $out/bin/dot > dot.1
    installManPage dot.1
  '';

  meta = with lib; {
    description = "High-performance dotfiles manager with git-like semantics";
    longDescription = ''
      dotman is a blazingly fast dotfiles manager built in Rust with
      SIMD acceleration, parallel processing, and content deduplication.

      Key features:
      - Git-like interface with familiar commands
      - SIMD-accelerated operations for maximum performance
      - Parallel file processing using all available CPU cores
      - Memory-mapped I/O for efficient large file handling
      - xxHash3 for ultra-fast file hashing (>1GB/s throughput)
      - Content-based deduplication and Zstd compression
      - Binary index format for instant loading

      This package provides the 'dot' command-line tool.
    '';
    homepage = "https://github.com/UtsavBalar1231/dotman";
    license = licenses.mit;
    maintainers = [ ];
    platforms = platforms.all;
    mainProgram = "dot";
  };
}
