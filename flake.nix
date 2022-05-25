# this flake was inspired by https://github.com/t1lde/rust-scaffold
# thanks t1lde 👍!
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    naersk = {
      url = "github:nmattia/naersk";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    pre-commit-hooks = {
      url = "github:cachix/pre-commit-hooks.nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = {
    self,
    nixpkgs,
    naersk,
    fenix,
    pre-commit-hooks,
    ...
  }: let
    supportedSystems = ["x86_64-linux"];

    # Read the current nightly version from a file
    nightly-version = builtins.readFile ./nightly-version;

    # forall a. (system -> a) -> { system => a }
    perSystem = nixpkgs.lib.genAttrs supportedSystems;

    # system -> nixpkgs-attrset
    nixpkgsFor = system: nixpkgs.legacyPackages."${system}";

    # system -> { string => fenix-package }
    fenix-packagesFor = system: fenix.packages."${system}";

    # An attrset with packages from the current toolchain snapshot
    # system -> rust-toolchain
    toolchainFor = system: let
      fenix-packages = fenix-packagesFor system;

      # The nightly toolchain variant to use.
      # You can also set this to 'minimal' or 'complete'
      fenix-toolchain = fenix-packages.default;
    in {
      inherit (fenix-toolchain) rustc cargo rustfmt;
      inherit (fenix-packages) rust-analyzer;
      inherit (fenix-packages.complete) rust-src;
    };

    # system -> naersk-lib
    naersk-libFor = system: let
      toolchain = toolchainFor system;
    in
      naersk.lib."${system}".override {
        inherit (toolchain) rustc cargo;
      };

    # Generate the Cargo.lock with 'cargo fetch' so we can provide it to the nix package
    mkCargoRoot = {
      toolchain,
      pkgs,
      src,
      name,
    }:
      pkgs.stdenv.mkDerivation {
        src = "${src}/Cargo.toml";
        name = "${name}-cargo-lock";
        buildInputs = with toolchain; [cargo rustc src];
        buildPhase = ''
          cp $src Cargo.toml
          cargo fetch
        '';
        installPhase = ''
          mkdir $out
          cp $src $out/
          cp Cargo.lock $out/
        '';
        phases = ["buildPhase" "installPhase"];
      };

    # Build various derivations relating to the rust project
    projectFor = system: let
      inherit (cargo-project.package) name;

      toolchain = toolchainFor system;
      pkgs = nixpkgsFor system;
      naersk-lib = naersk-libFor system;
      cargo-project = builtins.fromTOML (builtins.readFile ./Cargo.toml);
      cargo-root = mkCargoRoot {
        inherit toolchain pkgs name;
        src = ./.;
      };
    in rec {
      inherit name;

      # Environment-variables passed to cargo & dependency non-rust builds
      build-env = {
        # Any env-variables you may need (or want) for building
        # LIBCLANG_PATH = "${llvmPackages.libclang}/lib";
        # PROTOC = "${protobuf}/bin/protoc";
        # PROTOC_INCLUDE = "${protobuf}/include";
      };

      # The compiled output of the project
      package = naersk-lib.buildPackage (build-env
        // {
          inherit (cargo-project.package) version;

          src = ./.;
          root = cargo-root;
          pname = name;

          # Project build dependencies
          # Add non-rust dependencies here
          buildInputs = with pkgs; [
            toolchain.rustc
            toolchain.cargo
            libiconv
            # Common C deps
            # libclang
            # pkg-config
            # openssl.dev
          ];

          # Extra options, see https://github.com/nix-community/naersk
          doDoc = true; # docs
          copyLibs = true; # also link compiled library target in nix output
        });

      checks = {
        inherit package;
        pre-commit = pre-commit-hooks.lib.${system}.run {
          src = ./.;
          hooks = {
            alejandra.enable = true;
            statix.enable = true;
            clippy.enable = false; # 😱 error[E0514]: found crate `std` compiled by an incompatible version of rustc
            cargo-check.enable = true;
            hunspell.enable = false; # 😱 Can't open affix or dictionary files for dictionary named "default".
            rustfmt = {
              enable = true;
              entry = with toolchain;
                pkgs.lib.mkForce "${pkgs.writeShellApplication {
                  name = "check-rustfmt";
                  runtimeInputs = [rustfmt cargo];
                  text = "cargo-fmt fmt --manifest-path ${./.}/Cargo.toml -- --check --color always";
                }}/bin/check-rustfmt";
            };
          };
        };
      };

      # A devShell with some extras
      devShell = pkgs.mkShell (build-env
        // {
          # Include the buildInputs from the project
          inputsFrom = [
            package
          ];

          # Add any extras here
          buildInputs = with pkgs; [pre-commit];
          nativeBuildInputs = with toolchain; [
            rustfmt
            rust-analyzer
          ];

          # Extra env variables not used for the build
          RUST_ANALYZER_PATH = "${toolchain.rust-analyzer}";
          RUST_SRC_PATH = "${toolchain.rust-src}/lib/rustlib/src/rust/library";
          CARGO_PATH = "${toolchain.cargo}/bin/cargo";

          # Shell hook defining helper functions & symlinking the generated Cargo.lock
          shellHook =
            checks.pre-commit.shellHook
            + ''
              link-cargo-lock () {
                local lock=./Cargo.lock
                if test -L "$lock"; then
                  rm "$lock";
                elif test -e "$lock"; then
                  echo 'refusing to overwrite existing (non-symlinked) Cargo.lock'
                  exit 1
                fi
                ln -s ${cargo-root}/Cargo.lock "$lock"
              }
              update-nightly-version () {
                local next="$(date -I)"
                local prev="$(cat ./nightly-version)"
                echo 'updating nightly version in ./nightly-version: '"$prev -> $next"
                echo "$next" > ./nightly-version
              }
              link-cargo-lock
            '';
        });
    };

    # system -> { string => derivation }
    packagesFor = system: let
      project = projectFor system;
    in {
      "${project.name}" = project.package;
      default = project.devShell;
    };

    packages = perSystem packagesFor;

    # system -> shell-derivation
    devShellFor = system: let
      project = projectFor system;
    in {
      "${project.name}" = project.devShell;
      default = project.devShell;
    };
  in {
    inherit packages;

    # For nix repl & downstream
    toolchain = perSystem toolchainFor;
    naersk-lib = perSystem naersk-libFor;
    fenix-packages = perSystem fenix-packagesFor;
    project = perSystem projectFor;

    # nix flake check
    checks = perSystem (system: let project = projectFor system; in project.checks);

    # nix develop
    devShells = perSystem devShellFor;

    # nix fmt
    formatter = perSystem (system: let pkgs = nixpkgsFor system; in pkgs.alejandra);
  };
}
