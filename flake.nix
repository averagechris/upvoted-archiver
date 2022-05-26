{
  description = "TODO: archive ur upvotes";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.flake-utils.follows = "flake-utils";
    };
    pre-commit-hooks = {
      url = "github:cachix/pre-commit-hooks.nix";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.flake-utils.follows = "flake-utils";
    };
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = {
    self,
    flake-utils,
    nixpkgs,
    crane,
    pre-commit-hooks,
    fenix,
    ...
  }:
    flake-utils.lib.eachSystem [flake-utils.lib.system.x86_64-linux] (system: let
      pkgs = import nixpkgs {
        inherit system;
      };
      inherit (pkgs) lib;

      toolchain = {
        inherit (fenix.packages.${system}) rust-analyzer;
        inherit (fenix.packages.${system}.default) cargo rustc rustfmt;
        inherit (fenix.packages.${system}.complete) clippy rust-src;
      };
      craneLib = crane.lib.${system}.overrideScope' (final: prev: {
        inherit (toolchain) cargo clippy rust-analyzer rust-src rustc rustfmt;
      });

      common = rec {
        src = ./.;
        buildInputs = with pkgs; [openssl.dev] ++ builtins.attrValues toolchain;
        nativeBuildInputs = with pkgs; [gcc pkg-config toolchain.rustc];
        cargoArtifacts = craneLib.buildDepsOnly {
          inherit src buildInputs nativeBuildInputs;
        };
      };

      upvoted-archiver = craneLib.buildPackage common;

      clippy = craneLib.cargoClippy (common
        // {
          cargoClippyExtraArgs = "-- --deny warnings";
        });
    in {
      checks =
        {
          # Build the crate as part of `nix flake check` for convenience
          inherit upvoted-archiver;

          pre-commit = pre-commit-hooks.lib.${system}.run {
            inherit (common) src;
            hooks = {
              alejandra.enable = true;
              statix.enable = true;
              cargo-check = {
                enable = true;
                entry = pkgs.lib.mkForce "${pkgs.writeShellApplication {
                  name = "check-cargo-check";
                  runtimeInputs = upvoted-archiver.buildInputs ++ upvoted-archiver.nativeBuildInputs;
                  text = ''
                    CARGO_HOME=${common.cargoArtifacts.cargoVendorDir} \
                    PKG_CONFIG_PATH="${pkgs.openssl.dev}/lib/pkgconfig" \
                    cargo check --all-targets --profile=test
                  '';
                }}/bin/check-cargo-check";
              };
              rustfmt = {
                enable = true;
                entry = pkgs.lib.mkForce "${pkgs.writeShellApplication {
                  name = "check-rustfmt";
                  runtimeInputs =
                    (craneLib.cargoFmt common)
                    .nativeBuildInputs;
                  text = "cargo fmt";
                }}/bin/check-rustfmt";
              };
              clippy = let
                clippy-cmd = with pkgs.lib.strings; (removeSuffix "\n\nrunHook postBuild\n" (removePrefix "runHook preBuild\n" clippy.buildPhase));
              in {
                enable = true;
                entry = pkgs.lib.mkForce "${
                  pkgs.writeShellApplication {
                    name = "check-clippy";
                    runtimeInputs = clippy.buildInputs ++ clippy.nativeBuildInputs ++ upvoted-archiver.buildInputs ++ upvoted-archiver.nativeBuildInputs;
                    text = ''
                      export CARGO_HOME=${common.cargoArtifacts.cargoVendorDir}
                      export PKG_CONFIG_PATH="${pkgs.openssl.dev}/lib/pkgconfig"
                      ${clippy-cmd}
                    '';
                  }
                }/bin/check-clippy";
              };
            };
          };
        }
        // lib.optionalAttrs (system == "x86_64-linux") {
          # NB: cargo-tarpaulin only supports x86_64 systems
          # Check code coverage (note: this will not upload coverage anywhere)
          test-coverage = craneLib.cargoTarpaulin common;
        };

      packages.default = upvoted-archiver;

      apps.default = flake-utils.lib.mkApp {
        drv = upvoted-archiver;
      };

      devShells.default = with toolchain;
        pkgs.mkShell {
          inherit (self.checks.${system}.pre-commit) shellHook;
          inputsFrom = [upvoted-archiver clippy];
          buildInputs = with pkgs; [cachix];
          RUST_ANALYZER_PATH = "${rust-analyzer}";
          RUST_SRC_PATH = "${rust-src}/lib/rustlib/src/rust/library";
          CARGO_PATH = "${cargo}/bin/cargo";
        };

      formatter = pkgs.alejandra;
    });
}
