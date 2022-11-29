{
  inputs = {
    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.flake-compat.follows = "flake-compat";
      inputs.flake-utils.follows = "flake-utils";
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
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    pre-commit = {
      url = "github:cachix/pre-commit-hooks.nix";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.flake-compat.follows = "flake-compat";
      inputs.flake-utils.follows = "flake-utils";
      inputs.gitignore.follows = "gitignore";
    };
    rust = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.flake-utils.follows = "flake-utils";
    };
  };

  outputs = { self, crane, flake-utils, gitignore, nixpkgs, pre-commit, rust, ... }:
    let
      systems = [ "aarch64-linux" "x86_64-linux" ];
    in
    flake-utils.lib.eachSystem systems (hostPlatform:
      let
        targetPlatform = nixpkgs.lib.systems.examples.aarch64-multiplatform-musl;

        pkgs = import nixpkgs {
          localSystem = hostPlatform;
          crossSystem = targetPlatform;
          overlays = [ gitignore.overlay rust.overlays.default ];
        };

        rustToolchain = pkgs.pkgsBuildHost.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;

        craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;

        src = pkgs.gitignoreSource ./.;

        crateExpr = craneFn:
          { stdenv, qemu, gitignoreSource }:
          craneFn {
            inherit src;
            cargoArtifacts = craneLib.buildDepsOnly { inherit src; };
            depsBuildBuild = [ qemu ];
            nativeBuildInputs = [
              stdenv.cc
            ];
            HOST_CC = "${stdenv.cc.nativePrefix}cc";
            CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_LINKER = "${stdenv.cc.targetPrefix}cc";
            cargoClippyExtraArgs = "--all-targets -- --deny warnings";
          };
      in
      {
        checks = {
          inherit (self.packages.${hostPlatform}) http-spi-bridge;
          crate-fmt = pkgs.callPackage (crateExpr craneLib.cargoFmt) { };
          crate-clippy = pkgs.callPackage (crateExpr craneLib.cargoClippy) { };
          pre-commit = pre-commit.lib.${hostPlatform}.run {
            inherit src;
            hooks = {
              nixpkgs-fmt.enable = true;
              statix.enable = true;
            };
          };
        };

        packages = {
          default = self.packages.${hostPlatform}.http-spi-bridge;
          http-spi-bridge = pkgs.callPackage (crateExpr craneLib.buildPackage) { };
        };

        devShells.default = self.packages.${hostPlatform}.default.overrideAttrs (old: {
          name = "http-spi-bridge";
          src = null;
          version = null;
          depsBuildBuild = with pkgs.pkgsBuildBuild; (old.depsBuildBuild or [ ]) ++ [
            cargo-audit
            cargo-bloat
            cargo-edit
            cargo-outdated
            nixpkgs-fmt
            rnix-lsp
            rust-analyzer
            statix
          ];

          shellHook = (old.shellHook or "") + ''
            ${self.checks.${hostPlatform}.pre-commit.shellHook}
          '';
        });
      });
}
