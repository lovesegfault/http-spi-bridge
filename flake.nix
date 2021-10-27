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
        lib = nixpkgs.lib;

        crossSystem = lib.systems.examples.aarch64-multiplatform-musl;
        pkgs = import nixpkgs {
          inherit localSystem crossSystem;
          overlays = [ fenix.overlay gitignore.overlay ];
          crossOverlays = [
            (final: _: {
              rustToolchain = final.pkgsBuildHost.fenix.fromToolchainFile {
                file = ./rust-toolchain.toml;
                sha256 = "sha256-L1e0o7azRjOHd0zBa+xkFnxdFulPofTedSTEYZSjj2s=";
              };

              rustPlatform = final.makeRustPlatform {
                cargo = final.rustToolchain;
                rustc = final.rustToolchain;
              };
            })
          ];
        };

        systemToEnv = name: lib.replaceStrings [ "-" ] [ "_" ] (lib.toUpper name);
      in
      {
        packages.http-spi-bridge = pkgs.rustPlatform.buildRustPackage {
          name = "http-spi-bridge";

          src = pkgs.gitignoreSource ./.;

          cargoLock.lockFile = ./Cargo.lock;

          nativeBuildInputs = with pkgs.pkgsBuildHost; [ stdenv.cc ];

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

          CARGO_BUILD_TARGET = crossSystem.config;
          "CARGO_TARGET_${systemToEnv crossSystem.config}_LINKER" = "${crossSystem.config}-gcc";

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
