{
  description = "Earmark Rust workspace development shell";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ rust-overlay.overlays.default ];
        };

        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rustfmt" "clippy" "rust-analyzer" ];
        };

        earmark = pkgs.rustPlatform.buildRustPackage {
          pname = "em";
          version = "0.1.0";
          src = ./.;
          cargoLock = {
            lockFile = ./Cargo.lock;
          };
          nativeBuildInputs = with pkgs; [ pkg-config ];
          buildInputs = with pkgs; [ openssl ];

          # Tests require specific environment setup, handled by devShell or smoke scripts
          doCheck = false;

          meta = with pkgs.lib; {
            description = "Earmark operator shell";
            homepage = "https://github.com/Mikhail-Shakhnazarov/earmark-workspace";
            license = licenses.agpl3Plus;
            mainProgram = "em";
          };
        };
      in {
        packages.default = earmark;
        apps.default = flake-utils.lib.mkApp {
          drv = earmark;
        };

        devShells.default = pkgs.mkShell {
          packages = with pkgs; [
            rustToolchain
            pkg-config
            openssl
            openssl.dev
            git
            perl
          ];

          shellHook = ''
            export PKG_CONFIG_PATH="${pkgs.openssl.dev}/lib/pkgconfig:$PKG_CONFIG_PATH"
            export OPENSSL_DIR="${pkgs.openssl.dev}"
            export OPENSSL_LIB_DIR="${pkgs.openssl.out}/lib"
            export OPENSSL_INCLUDE_DIR="${pkgs.openssl.dev}/include"
            export OPENSSL_NO_VENDOR="1"

            echo "Earmark nix dev shell ready (Rust + OpenSSL + pkg-config)"
            echo "Try: cargo check --workspace && cargo test --workspace"
          '';
        };
      });
}
