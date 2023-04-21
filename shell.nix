{ pkgs ? import (fetchTarball "https://github.com/NixOS/nixpkgs/archive/645bc49f34fa8eff95479f0345ff57e55b53437e.tar.gz") {} }:

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
  ];
}
