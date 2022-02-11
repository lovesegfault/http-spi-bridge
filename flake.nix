{
  inputs = {
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-compat = {
      url = "github:edolstra/flake-compat";
      flake = false;
    };
    flake-utils.url = "github:numtide/flake-utils";
    gitignore = {
      url = "github:hercules-ci/gitignore.nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    naersk = {
      url = "github:nix-community/naersk";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable-small";
  };

  outputs = { self, fenix, flake-utils, gitignore, naersk, nixpkgs, ... }:
    flake-utils.lib.eachDefaultSystem (localSystem:
      let
        crossSystem = nixpkgs.lib.systems.examples.aarch64-multiplatform-musl // { useLLVM = true; };

        pkgs = import nixpkgs {
          inherit localSystem crossSystem;
          overlays = [ fenix.overlay gitignore.overlay naersk.overlay ];
        };

        inherit (pkgs) pkgsBuildBuild pkgsBuildHost;

        llvmToolchain = pkgsBuildHost.llvmPackages_latest;

        rustToolchain = pkgsBuildHost.fenix.fromToolchainFile {
          file = ./rust-toolchain.toml;
          sha256 = "sha256-NL+YHnOj1++1O7CAaQLijwAxKJW9SnHg8qsiOJ1m0Kk=";
        };

        naerskCross = pkgsBuildHost.naersk.override {
          inherit (llvmToolchain) stdenv;
          cargo = rustToolchain;
          rustc = rustToolchain;
        };

        src = pkgs.gitignoreSource ./.;
      in
      {
        packages.http-spi-bridge = naerskCross.buildPackage {
          name = "http-spi-bridge";

          inherit src;

          nativeBuildInputs = with llvmToolchain; [ stdenv.cc lld ];
        };

        defaultPackage = self.packages.${localSystem}.http-spi-bridge;

        devShell = pkgs.mkShell {
          name = "http-spi-bridge";

          inputsFrom = [ self.defaultPackage.${localSystem} ];

          nativeBuildInputs = with pkgsBuildBuild; [
            cargo-audit
            cargo-bloat
            cargo-edit
            cargo-udeps
            file
            nix-linter
            nixpkgs-fmt
            pre-commit
            rnix-lsp
            rust-analyzer-nightly
            (pkgs.lib.lowPrio git)
          ];

          shellHook = ''
            pre-commit install --install-hooks
          '';
        };
      });
}
