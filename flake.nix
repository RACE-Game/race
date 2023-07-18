{
  description = "Race protocol flake";

  inputs = {
    rust-overlay.url = "github:oxalica/rust-overlay";
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { nixpkgs, flake-utils, rust-overlay, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        code = pkgs.callPackage ./. { inherit nixpkgs system rust-overlay; };
      in rec {
        packages = {
          race-transactor = code.race-transactor;
          race-cli = code.race-cli;
          all = pkgs.symlinkJoin {
            name = "all";
            paths = with code; [ race-transactor race-cli ];
          };
        };

        default = packages.all;

        devShell = pkgs.mkShell {
          nativeBuildInputs = with pkgs; [
            (rust-bin.stable.latest.default.override {
              extensions = [ "rust-src" ];
              targets = [ "wasm32-unknown-unknown" ];
            })
            cargo
            openssl
            pkg-config
            rust-analyzer
            simple-http-server
            solana-cli
            nodejs_18
            just
            binaryen
            nodePackages.typescript-language-server
          ];
          RUST_LOG = "info,wasmer_compiler_cranelift=error,solana_rpc_client=error";
          RUST_BACKTRACE = 1;
        };
      }
    );

  nixConfig = {
    bash-prompt-prefix = "[race]";
  };
}
