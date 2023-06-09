# Nixpgs unstable branch
{ pkgs ? import (fetchTarball "https://github.com/NixOS/nixpkgs/archive/a558f7ac29f50c4b937fb5c102f587678ae1c9fb.tar.gz") { } }:

pkgs.mkShell rec {
  buildInputs = with pkgs; [
    rustup
    wasm-pack
    openssl
    rust-analyzer
    simple-http-server
    # Current LST
    nodejs_18
    just
    git
    tokei
    rnix-lsp
    nixpkgs-fmt
    binaryen
  ];
  RUST_LOG = "info,wasmer_compiler_cranelift=error,solana_rpc_client=error";
  RUST_BACKTRACE = 1;
}
