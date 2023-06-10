{
  description = "Race protocol flake";

  inputs = {
    nixpkgs = { url = "github:NixOS/nixpkgs/nixpkgs-unstable"; };
    flake-utils = { url = "github:numtide/flake-utils"; };
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
      in
        {
          devShell = pkgs.mkShell {
            buildInputs = with pkgs; [
              rustup
              wasm-pack
              openssl
              rust-analyzer
              simple-http-server
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
          };
        }
    );

  nixConfig = {
    bash-prompt-prefix = "[race]";
  };
}
