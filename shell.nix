{ pkgs ? import (fetchTarball "https://github.com/NixOS/nixpkgs/archive/645bc49f34fa8eff95479f0345ff57e55b53437e.tar.gz") { } }:

pkgs.mkShell {
  buildInputs = with pkgs; [
    rustup
    rust-analyzer
    wasm-pack
    nodejs-16_x
    solana-validator
    just
    git
    tokei
    rnix-lsp
    nixpkgs-fmt
  ];
  RUST_LOG = "info,wasmer_compiler_cranelift=error,solana_rpc_client=error";
  RUST_BACKTRACE = 1;
}
