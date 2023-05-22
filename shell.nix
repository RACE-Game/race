{ pkgs ? import (fetchTarball "https://github.com/NixOS/nixpkgs/archive/645bc49f34fa8eff95479f0345ff57e55b53437e.tar.gz") { } }:

pkgs.mkShell rec {
  buildInputs = with pkgs; [
    rustup
    wasm-pack
    openssl
    rust-analyzer
    simple-http-server
    nodejs-16_x
    just
    git
    tokei
    rnix-lsp
    nixpkgs-fmt
    binaryen
  ];
  RUST_LOG = "info,wasmer_compiler_cranelift=error,solana_rpc_client=error,jsonrpsee_server=debug";
  RUST_BACKTRACE = 1;
}
