{
  inputs = {
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils.url = "github:numtide/flake-utils";
    gitignore = {
      url = "github:hercules-ci/gitignore.nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    pre-commit-hooks = {
      url = "github:cachix/pre-commit-hooks.nix";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.flake-utils.follows = "flake-utils";
    };
  };

  outputs = { self, fenix, flake-utils, gitignore, nixpkgs, pre-commit-hooks }:
    flake-utils.lib.eachDefaultSystem (localSystem:
      let
        pkgs = import nixpkgs {
          inherit localSystem;
          crossSystem = nixpkgs.lib.systems.examples.aarch64-multiplatform-musl;
          overlays = [ fenix.overlay gitignore.overlay ];
          crossOverlays = [
            (final: _: {
              rustToolchain = with final.pkgsBuildHost.fenix; combine [
                stable.rustc
                stable.cargo
                stable.rust-src
                stable.clippy
                stable.rustfmt
                targets.${final.stdenv.targetPlatform.config}.stable.rust-std
              ];

              rustPlatform = final.makeRustPlatform {
                cargo = final.rustToolchain;
                rustc = final.rustToolchain;
              };
            })
          ];
        };
      in
      {
        packages.http-spi-bridge = pkgs.rustPlatform.buildRustPackage {
          name = "http-spi-bridge";

          src = pkgs.gitignoreSource ./.;

          cargoLock.lockFile = ./Cargo.lock;

          RUSTFLAGS = "-C target-feature=+crt-static";
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
            rust-analyzer-nightly
            file
            nix-linter
            nixpkgs-fmt
            qemu
          ];

          inherit (self.defaultPackage.${localSystem}) RUSTFLAGS;
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
