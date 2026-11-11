{
  description = "Nix Flake for EDCAS (Elite Dangerous Commander Assistant System)";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    edcas-src = {
      url = "https://edcas.de/download/edcas-client-source.tar.gz";
      flake = false;
    };
    edcas-bin = {
      url = "file+https://edcas.de/download/edcas-client-linux-x86_64";
      flake = false;
    };
    edcas-3dmigoto = {
      url = "file+https://edcas.de/download/edcas-client-3dmigoto-x64.zip";
      flake = false;
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      edcas-src,
      edcas-bin,
      edcas-3dmigoto,
    }:
    let
      supportedSystems = [
        "x86_64-linux"
        "aarch64-linux"
      ];
      forAllSystems = nixpkgs.lib.genAttrs supportedSystems;
      pkgsFor = system: nixpkgs.legacyPackages.${system};
    in
    {
      packages = forAllSystems (
        system:
        let
          pkgs = pkgsFor system;
        in
        {
          default = self.packages.${system}.edcas-client;
          edcas-client = pkgs.callPackage ./source {
            src = edcas-src;
            src3dmigoto = edcas-3dmigoto;
          };
          edcas-client-bin = pkgs.lib.optionalAttrs (system == "x86_64-linux") (
            pkgs.callPackage ./bin {
              srcBinary = edcas-bin;
              srcAssets = edcas-src;
            }
          );
          bin = self.packages.${system}.edcas-client-bin;
        }
      );

      apps = forAllSystems (system: {
        default = {
          type = "app";
          program = "${self.packages.${system}.default}/bin/edcas-client";
          meta = {
            description = "Run EDCAS client (built from source)";
          };
        };
        bin = {
          type = "app";
          program = "${self.packages.${system}.bin}/bin/edcas-client";
          meta = {
            description = "Run EDCAS client (pre-built binary)";
          };
        };
      });

      overlays.default = final: prev: {
        edcas-client = self.packages.${prev.system}.edcas-client;
        edcas-client-bin = self.packages.${prev.system}.edcas-client-bin;
      };

      devShells = forAllSystems (
        system:
        let
          pkgs = pkgsFor system;
        in
        {
          default = pkgs.mkShell {
            name = "edcas-client-dev";
            nativeBuildInputs = with pkgs; [
              cargo
              rustc
              rust-analyzer
              clippy
              rustfmt
              pkg-config
              rustPlatform.bindgenHook
            ];
            buildInputs = with pkgs; [
              openssl
              speechd
              libxkbcommon
              libglvnd
              wayland
              libx11
              libxcursor
              libxrandr
              libxi
            ];
            LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath (
              with pkgs;
              [
                libglvnd
                libxkbcommon
                wayland
                libx11
                libxcursor
                libxrandr
                libxi
                speechd
              ]
            );
          };
        }
      );
    };
}
