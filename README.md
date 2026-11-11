# EDCAS Nix Flake

Nix Flake packaging for **[EDCAS](https://edcas.de)** (*Elite Dangerous Commander Assistant System*) — a companion desktop application for Elite Dangerous written in Rust (egui/eframe).

This repository provides two packaging options:
1. **Source build (`default` / `edcas-client`)**: Built from source with `rustPlatform.buildRustPackage`.
2. **Pre-compiled binary (`bin` / `edcas-client-bin`)**: Fast installation of the upstream pre-built x86_64 Linux binary via `autoPatchelfHook`.

Both packages include full desktop launcher integration (`.desktop` and icons) and runtime wrappers for OpenGL, Wayland, X11 and speech-dispatcher.

---

## Quickstart

Run directly without installing:

```sh
# Run the pre-compiled binary (instant)
nix run github:Djabx/edca-flake#bin

# Or build and run from source
nix run github:Djabx/edca-flake
```

---

## NixOS Configuration

Add `edca-flake` to your `flake.nix`:

```nix
{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    edcas.url = "github:Djabx/edca-flake";
    edcas.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs = { self, nixpkgs, edcas, ... }: {
    nixosConfigurations.myhostname = nixpkgs.lib.nixosSystem {
      system = "x86_64-linux";
      modules = [
        ({ pkgs, ... }: {
          # Use pre-built binary:
          environment.systemPackages = [ edcas.packages.${pkgs.system}.bin ];

          # Or compile from source:
          # environment.systemPackages = [ edcas.packages.${pkgs.system}.default ];
        })
      ];
    };
  };
}
```

---

## Development Shell

If you are developing or testing `edcas-client`, enter a complete shell with Rust, rust-analyzer, and all required C libraries:

```sh
nix develop
cargo build --release -p edcas-client
```

---

## Repository Structure

```text
.
├── flake.nix              # Main Flake definition (packages, apps, devShell, overlay)
├── source/
│   └── default.nix        # Source-based derivation (buildRustPackage)
├── bin/
│   └── default.nix        # Binary-based derivation (autoPatchelfHook)
└── .github/workflows/     # CI validation
```

---

## Notes for Upstream

This flake can easily be adopted by upstream:
- The `source/default.nix` derivation can be called directly on the upstream source tree via `pkgs.callPackage ./source { src = ./.; }`.
- For `nixpkgs` inclusion, releases need to be permanently archived with versioned URLs (e.g. `edcas-client-source-157.tar.gz`) or Git release tags.
