{
  inputs = {
    fenix = {
      url = "github:figsoda/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.naersk.follows = "naersk";
    };
    flake-utils.url = "github:numtide/flake-utils";
    naersk = {
      url = "github:nmattia/naersk";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    pre-commit-hooks = {
      url = "github:cachix/pre-commit-hooks.nix";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.flake-utils.follows = "flake-utils";
    };
  };

  outputs = { self, fenix, flake-utils, naersk, nixpkgs, pre-commit-hooks }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs =
          if "${system}" == "aarch64-linux" then
            nixpkgs.legacyPackages.${system}
          else
            nixpkgs.legacyPackages.${system}.pkgsCross.aarch64-multiplatform-musl;

        fenixPkgs = fenix.packages.${system};

        target = "aarch64-unknown-linux-musl";

        rustFull = with fenixPkgs; combine [
          (latest.withComponents [
            "cargo"
            "clippy-preview"
            "rust-src"
            "rust-std"
            "rustc"
            "rustfmt-preview"
          ])
          targets.${target}.latest.rust-std
        ];

        naerskBuild = (naersk.lib.${system}.override {
          stdenv = pkgs.llvmPackages_11.stdenv;
          cargo = rustFull;
          rustc = rustFull;
        }).buildPackage;

        cargoConfig = {
          CARGO_BUILD_TARGET = target;
          RUSTFLAGS = "-C linker-flavor=ld.lld -C target-feature=+crt-static";
        };
      in
      {
        packages.http-spi-bridge = naerskBuild ({
          src = ./.;

          doDoc = true;

          nativeBuildInputs = with pkgs.pkgsBuildBuild.llvmPackages_11; [ clang lld ];

          dontPatchELF = true;
        } // cargoConfig);

        defaultPackage = self.packages.${system}.http-spi-bridge;

        devShell = pkgs.mkShell ({
          inherit (self.checks.${system}.pre-commit-check) shellHook;
          name = "http-spi-bridge";

          nativeBuildInputs = (self.defaultPackage.${system}.nativeBuildInputs or [ ])
            ++ (with pkgs.pkgsBuildBuild; [
            cargo-edit
            cargo-udeps
            fenixPkgs.rust-analyzer
            file
            nix-linter
            nixpkgs-fmt
            qemu
          ]);
        } // cargoConfig);

        checks.pre-commit-check = (pre-commit-hooks.lib.${system}.run {
          src = ./.;
          hooks = {
            nix-linter.enable = true;
            nixpkgs-fmt.enable = true;
            rustfmt.enable = true;
          };
        });
      });
}
