{
  description = "Classical music in your terminal.";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs { inherit system; };

        nativeBuildInputs = [ pkgs.pkg-config ];
        buildInputs = [
          pkgs.mpv
          pkgs.openssl
        ];

        classfi = pkgs.rustPlatform.buildRustPackage {
          pname = "classfi";
          version = "0.2.1";

          src = ./.;

          cargoLock.lockFile = ./Cargo.lock;

          inherit nativeBuildInputs buildInputs;

          meta = {
            description = "Classical music in your terminal.";
            homepage = "https://github.com/carmiac/classfi";
            license = pkgs.lib.licenses.gpl3Plus;
            mainProgram = "classfi";
          };
        };
      in
      {
        packages.default = classfi;

        apps.default = flake-utils.lib.mkApp { drv = classfi; };

        devShells.default = pkgs.mkShell {
          inherit buildInputs;
          nativeBuildInputs = nativeBuildInputs ++ [
            pkgs.cargo
            pkgs.rustc
            pkgs.rust-analyzer
            pkgs.clippy
            pkgs.rustfmt
          ];
        };
      }
    );
}
