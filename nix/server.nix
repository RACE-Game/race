# An example shell.nix for server environment

{ pkgs ? import (fetchTarball "https://github.com/NixOS/nixpkgs/archive/645bc49f34fa8eff95479f0345ff57e55b53437e.tar.gz") {} }:

pkgs.mkShell {
  buildInputs = with pkgs; [
    openssl
  ];
  RUST_LOG = "info,wasmer_compiler_cranelift=error,solana_rpc_client=error";
  RUST_BACKTRACE = 1;
}
