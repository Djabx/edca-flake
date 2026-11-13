{
  description = "Nix Flake for EDCAS (Elite Dangerous Commander Assistant System)";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
  };

  outputs =
    {
      self,
      nixpkgs,
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
          edcas-client = pkgs.callPackage ./package.nix { };
        }
      );

      apps = forAllSystems (system: {
        default = {
          type = "app";
          program = "${self.packages.${system}.default}/bin/edcas-client";
          meta = {
            description = "Run EDCAS client";
          };
        };
      });

      overlays.default = final: prev: {
        edcas-client = self.packages.${prev.system}.edcas-client;
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
