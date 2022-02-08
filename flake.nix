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
    pre-commit-hooks = {
      url = "github:cachix/pre-commit-hooks.nix";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.flake-utils.follows = "flake-utils";
    };
  };

  outputs = { self, fenix, flake-utils, gitignore, naersk, nixpkgs, pre-commit-hooks, ... }:
    flake-utils.lib.eachDefaultSystem (localSystem:
      let
        crossSystem = nixpkgs.lib.systems.examples.aarch64-multiplatform-musl // { useLLVM = true; };
        pkgs = import nixpkgs {
          inherit localSystem crossSystem;
          overlays = [
            fenix.overlay
            gitignore.overlay
            naersk.overlay
            (final: prev: {
              rustToolchainCfg = {
                file = ./rust-toolchain.toml;
                sha256 = "sha256-NL+YHnOj1++1O7CAaQLijwAxKJW9SnHg8qsiOJ1m0Kk=";
              };

              rustToolchain = final.fenix.combine [
                (final.pkgsBuildHost.fenix.fromToolchainFile final.rustToolchainCfg)
                (final.fenix.targets.${crossSystem.config}.fromToolchainFile final.rustToolchainCfg)
              ];

              rustStdenv = final.pkgsBuildHost.llvmPackages_13.stdenv;
              rustLinker = final.pkgsBuildHost.llvmPackages_13.lld;

              naerskBuild = (prev.pkgsBuildHost.naersk.override {
                cargo = final.rustToolchain;
                rustc = final.rustToolchain;
                stdenv = final.rustStdenv;
              }).buildPackage;
            })
          ];
        };
      in
      {
        packages.http-spi-bridge = pkgs.naerskBuild {
          name = "http-spi-bridge";

          src = pkgs.gitignoreSource ./.;

          nativeBuildInputs = with pkgs; [ rustStdenv.cc rustLinker ];

          CARGO_BUILD_TARGET = crossSystem.config;

          RUSTFLAGS = "-C linker-flavor=ld.lld -C target-feature=+crt-static";
        };

        defaultPackage = self.packages.${localSystem}.http-spi-bridge;

        devShell = pkgs.mkShell {
          name = "http-spi-bridge";

          inputsFrom = [ self.defaultPackage.${localSystem} ];

          nativeBuildInputs = with pkgs.pkgsBuildBuild; [
            cargo-audit
            cargo-bloat
            cargo-edit
            cargo-udeps
            file
            nix-linter
            nixpkgs-fmt
            rnix-lsp
            rust-analyzer-nightly
          ];

          inherit (self.defaultPackage.${localSystem}) CARGO_BUILD_TARGET RUSTFLAGS;
          inherit (self.checks.${localSystem}.pre-commit-check) shellHook;
        };

        checks.pre-commit-check = (pre-commit-hooks.lib.${localSystem}.run {
          src = ./.;
          hooks = {
            nix-linter.enable = true;
            nixpkgs-fmt.enable = true;
          };
        });
      });
}
