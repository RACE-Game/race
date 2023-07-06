{ pkgs, ... }:
{
  race-transactor = pkgs.rustPlatform.buildRustPackage {
    pname = "race-transactor";
    version = "0.0.4";
    src = ./.;
    cargoBuildFlags = [ "-p" "race-transactor" ];

    cargoLock = {
      lockFile = ./Cargo.lock;
    };

    buildInputs = with pkgs; [ openssl ];
    nativeBuildInputs = with pkgs; [ pkg-config ];
  };
  race-cli = pkgs.rustPlatform.buildRustPackage {
    pname = "race-cli";
    version = "0.0.4";
    src = ./.;
    cargoBuildFlags = [ "-p" "race-cli" ];

    cargoLock = {
      lockFile = ./Cargo.lock;
    };

    buildInputs = with pkgs; [ openssl ];
    nativeBuildInputs = with pkgs; [ pkg-config ];
  };
}
