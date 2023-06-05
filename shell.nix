# Nixpgs unstable branch
{ pkgs ? import (fetchTarball "https://github.com/NixOS/nixpkgs/archive/dd4982554e18b936790da07c4ea2db7c7600f283.tar.gz") { } }:

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
