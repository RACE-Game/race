{
  description = "Dev scripts for Race Protocol";

  inputs = {
    nixpkgs = { url = "github:NixOS/nixpkgs/nixos-23.05"; };
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
              just
              zellij
            ];
          };
        }
    );

  nixConfig = {
    bash-prompt-prefix = "[flake]";
  };
}
