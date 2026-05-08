{
  description = "Earmark workspace - Declarative context and execution kernel for governed AI work";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    crane.url = "github:ipetkov/crane";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
    advisory-db = {
      url = "github:rustsec/advisory-db";
      flake = false;
    };
  };

  outputs = { self, nixpkgs, crane, flake-utils, rust-overlay, advisory-db, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (rust-overlay.overlays.default) ];
        };
        rustVersion = "1.95.0";
        rustToolchain = pkgs.rust-bin.stable.${rustVersion}.default;

        craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;

        src = craneLib.cleanCargoSource ./.;

        commonArgs = {
          inherit src;
          pname = "earmark-workspace";  # Fix crane warnings
          strictDeps = true;

          nativeBuildInputs = with pkgs; [
            cmake clang libclang
          ];

          buildInputs = with pkgs; [
            git
            pkg-config
            openssl
            protobuf
          ] ++ lib.optionals stdenv.isDarwin [
            libiconv
          ];

          cargoExtraArgs = "--workspace";
        };

        cargoArtifacts = craneLib.buildDepsOnly commonArgs;
      in
      {
        checks = {
          workspace-clippy = craneLib.cargoClippy (commonArgs // {
            inherit cargoArtifacts;
            cargoClippyExtraArgs = "--all-targets -- -D warnings";
          });

          workspace-fmt = craneLib.cargoFmt { inherit src; };

          workspace-audit = craneLib.cargoAudit {
            inherit src advisory-db;
          };
        };

        packages.default = craneLib.buildPackage (commonArgs // {
          inherit cargoArtifacts;
          cargoExtraArgs = "-p earmark-cli";
        });

        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            rustToolchain
            git
            pkg-config
            openssl
            protobuf
          ];

          # Environment variables
          RUST_BACKTRACE = "1";
          RUST_LOG = "info";

          # Shell hook
          shellHook = ''
            echo "Entering Earmark development shell..."
            echo "Rust version: $(rustc --version)"
            echo "Cargo version: $(cargo --version)"
          '';
        };
      }
    );
}
