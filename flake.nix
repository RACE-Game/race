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
      in {
        devShell = pkgs.mkShell {
          nativeBuildInputs = with pkgs; [
            (rust-bin.stable."1.83.0".default.override {
              extensions = [ "rust-src" ];
              targets = [ "wasm32-unknown-unknown" ];
            })
            cargo
            openssl
            pkg-config
            nodejs_18
            just
            binaryen
            # For development
            rust-analyzer
            nodePackages.typescript
            nodePackages.typescript-language-server
            nodePackages.prettier
            zellij
          ];
          RUST_LOG = "info,hyper=error,parse_headers=error,encode_headers=error,wasmer_compiler_cranelift=info,solana_rpc_client=debug,solana_client=debug,jsonrpsee_server=info";
          RUST_BACKTRACE = 1;
        };
      }
    );
}
