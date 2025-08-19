{
  description = "dotman - Blazingly fast dotfiles manager with git-like semantics";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
    naersk.url = "github:nix-community/naersk";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils, naersk }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        # Use the Rust toolchain specified in the project
        rustToolchain = pkgs.rust-bin.stable."1.70.0".default.override {
          extensions = [ "rust-src" "clippy" "rustfmt" ];
        };

        naersk-lib = pkgs.callPackage naersk {
          cargo = rustToolchain;
          rustc = rustToolchain;
        };

        # Common build inputs
        buildInputs = with pkgs; [
          # Runtime dependencies
        ] ++ lib.optionals stdenv.isDarwin [
          # macOS specific dependencies
          darwin.apple_sdk.frameworks.Security
          darwin.apple_sdk.frameworks.SystemConfiguration
        ];

        nativeBuildInputs = with pkgs; [
          rustToolchain
          pkg-config
          installShellFiles
          help2man
        ];

      in
      {
        # Default package
        packages.default = naersk-lib.buildPackage {
          pname = "dotman";
          version = "0.0.1";

          src = ./../..;  # Root of the project

          inherit buildInputs nativeBuildInputs;

          # Build configuration
          doCheck = true;
          cargoOptions = (x: x ++ [ "--all-features" "--release" ]);

          # Override build phase to add optimizations
          RUSTFLAGS = "-C target-cpu=native -C link-arg=-s";

          # Post-install phase to add shell completions and man page
          postInstall = ''
            # Generate shell completions
            installShellCompletion --cmd dot \
              --bash <($out/bin/dot completion bash) \
              --zsh <($out/bin/dot completion zsh) \
              --fish <($out/bin/dot completion fish)

            # Generate man page
            help2man --no-info --name="blazingly fast dotfiles manager" \
              --version-string="0.0.1" \
              $out/bin/dot > dot.1
            installManPage dot.1
          '';

          meta = with pkgs.lib; {
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
            '';
            homepage = "https://github.com/UtsavBalar1231/dotman-rs";
            license = licenses.mit;
            maintainers = [ "UtsavBalar1231" ];
            platforms = platforms.all;
            mainProgram = "dot";
          };
        };

        # Development environment
        devShells.default = pkgs.mkShell {
          inputsFrom = [ self.packages.${system}.default ];
          buildInputs = with pkgs; [
            # Development tools
            cargo-audit
            cargo-deny
            cargo-edit
            cargo-watch
            cargo-nextest
            cargo-llvm-cov

            # Benchmarking and profiling
            cargo-criterion
            cargo-flamegraph

            # Documentation
            mdbook

            # Additional development utilities
            just
            direnv

            # Cross-compilation support
            pkgsCross.aarch64-multiplatform.stdenv.cc
            pkgsCross.mingwW64.stdenv.cc
          ];

          shellHook = ''
            echo "dotman development environment"
            echo "Available commands:"
            echo "  cargo build --release     - Build optimized binary"
            echo "  cargo test                - Run test suite"
            echo "  cargo bench               - Run benchmarks"
            echo "  just build                - Build using justfile"
            echo "  just test                 - Test using justfile"
          '';
        };

        # CI shell for GitHub Actions
        devShells.ci = pkgs.mkShell {
          buildInputs = with pkgs; [
            rustToolchain
            pkg-config
            cargo-audit
            cargo-deny
            help2man
          ];
        };

        # Cross-compilation packages
        packages.dotman-aarch64 = naersk-lib.buildPackage {
          pname = "dotman";
          version = "0.0.1";
          src = ./../..;

          inherit nativeBuildInputs;
          buildInputs = with pkgs.pkgsCross.aarch64-multiplatform; [
            stdenv.cc.libc
          ];

          CARGO_BUILD_TARGET = "aarch64-unknown-linux-gnu";
          CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER =
            "${pkgs.pkgsCross.aarch64-multiplatform.stdenv.cc}/bin/aarch64-unknown-linux-gnu-gcc";

          doCheck = false; # Skip tests for cross-compilation
        };

        # NixOS module
        nixosModules.default = { config, lib, pkgs, ... }:
          with lib;
          let
            cfg = config.services.dotman;
          in
          {
            options.services.dotman = {
              enable = mkEnableOption "dotman dotfiles manager";

              package = mkOption {
                type = types.package;
                default = self.packages.${pkgs.system}.default;
                description = "The dotman package to use.";
              };
            };

            config = mkIf cfg.enable {
              environment.systemPackages = [ cfg.package ];
            };
          };

        # Home Manager module
        homeManagerModules.default = { config, lib, pkgs, ... }:
          with lib;
          let
            cfg = config.programs.dotman;
          in
          {
            options.programs.dotman = {
              enable = mkEnableOption "dotman dotfiles manager";

              package = mkOption {
                type = types.package;
                default = self.packages.${pkgs.system}.default;
                description = "The dotman package to use.";
              };

              settings = mkOption {
                type = types.attrs;
                default = {};
                description = "Configuration for dotman.";
              };
            };

            config = mkIf cfg.enable {
              home.packages = [ cfg.package ];

              home.file.".config/dotman/config" = mkIf (cfg.settings != {}) {
                text = generators.toTOML {} cfg.settings;
              };
            };
          };

        # Formatter for `nix fmt`
        formatter = pkgs.nixpkgs-fmt;

        # Apps for `nix run`
        apps.default = flake-utils.lib.mkApp {
          drv = self.packages.${system}.default;
        };

        # Checks for `nix flake check`
        checks = {
          package = self.packages.${system}.default;

          # Additional checks
          clippy = naersk-lib.buildPackage {
            pname = "dotman-clippy";
            version = "0.0.1";
            src = ./../..;

            inherit buildInputs nativeBuildInputs;

            mode = "clippy";
            doCheck = false;
          };

          fmt = naersk-lib.buildPackage {
            pname = "dotman-fmt";
            version = "0.0.1";
            src = ./../..;

            inherit buildInputs nativeBuildInputs;

            mode = "fmt";
            doCheck = false;
          };
        };
      });
}
