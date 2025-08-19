# Traditional Nix expression for dotman
# Compatible with older Nix versions that don't support flakes

{ lib
, stdenv
, fetchFromGitHub
, rustPlatform
, pkg-config
, installShellFiles
, help2man
, Security
, SystemConfiguration
}:

rustPlatform.buildRustPackage rec {
  pname = "dotman";
  version = "0.0.1";

  src = fetchFromGitHub {
    owner = "UtsavBalar1231";
    repo = pname;
    rev = "v${version}";
    sha256 = lib.fakeSha256; # Will be updated by maintainers
  };

  cargoSha256 = lib.fakeSha256; # Will be updated by maintainers

  nativeBuildInputs = [
    pkg-config
    installShellFiles
    help2man
  ];

  buildInputs = lib.optionals stdenv.isDarwin [
    Security
    SystemConfiguration
  ];

  # Build with all features enabled
  buildFeatures = [ "default" ];

  # Use release profile for optimal performance
  buildType = "release";

  # Enable optimizations
  RUSTFLAGS = "-C target-cpu=native";

  # Run tests during build
  doCheck = true;

  # Post-install phase to add shell completions and man page
  postInstall = ''
    # Generate shell completions
    installShellCompletion --cmd dot \
      --bash <($out/bin/dot completion bash) \
      --zsh <($out/bin/dot completion zsh) \
      --fish <($out/bin/dot completion fish)
    
    # Generate man page
    help2man --no-info --name="blazingly fast dotfiles manager" \
      --version-string="${version}" \
      $out/bin/dot > dot.1
    installManPage dot.1
  '';

  meta = with lib; {
    description = "Blazingly fast dotfiles manager with git-like semantics";
    longDescription = ''
      dotman is a high-performance dotfiles manager designed for developers who
      demand speed without sacrificing functionality. Unlike traditional dotfile
      managers that treat performance as an afterthought, dotman is built from
      the ground up with extreme optimization in mind.
      
      Key features:
      - SIMD-accelerated operations for maximum performance
      - Parallel file processing using all available CPU cores
      - Memory-mapped I/O for efficient large file handling
      - xxHash3 for ultra-fast file hashing (>1GB/s throughput)
      - Sub-millisecond operations for typical repositories
      - Content-based deduplication and Zstd compression
      - Binary index format for instant loading
      - Git-like interface with familiar commands
      - Cross-platform support with architecture optimizations
      
      This package provides the 'dot' command-line tool.
    '';
    homepage = "https://github.com/UtsavBalar1231/dotman-rs";
    license = licenses.mit;
    maintainers = with maintainers; [ ];
    platforms = platforms.all;
    mainProgram = "dot";
  };
}