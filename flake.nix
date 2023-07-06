{
  description = "Race protocol flake";

  inputs = {
    rust-overlay.url = "github:oxalica/rust-overlay";
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = inputs: with inputs;
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        code = pkgs.callPackage ./. { inherit nixpkgs system rust-overlay; };
      in {
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
          buildInputs = with pkgs; [
            openssl
            rustc
            cargo
            pkg-config
            rust-analyzer
            simple-http-server
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
