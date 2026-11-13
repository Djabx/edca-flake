# EDCAS Nix Flake

Nix Flake packaging for **[EDCAS](https://edcas.de)** (*Elite Dangerous Commander Assistant System*) — a companion desktop application for Elite Dangerous written in Rust (egui/eframe).

This repository mirrors the upstream source code and builds the application from source using `nixpkgs` and `rustPlatform.buildRustPackage`. It also embeds the [3DMigoto](https://github.com/boates/3DMigoto-edcas) shader mod bundle directly into the built application for easy installation.

The package includes full desktop launcher integration (`.desktop` and icons) and runtime wrappers for OpenGL, Wayland, X11 and speech-dispatcher.

---

	## Quickstart

Run directly without installing:

```sh
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
          environment.systemPackages = [ edcas.packages.${pkgs.system}.default ];
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
cargo check --workspace
cargo build --release -p edcas-client
```

---

## Repository Structure

```text
.
└‰‰ flake.nix             # Main Flake definition (packages, apps, devShell, overlay)
└‰‰ package.nix           # Source-based derivation (buildRustPackage)
└‰‰ 3dmigoto/             # Embedded 3DMigoto zip bundle
└‰‰ Cargo.toml            # Upstream Cargo workspace
└‰‰ crates/               # Upstream Rust source
└‰‰ assets/               # Upstream assets
└‰‰ .github/workflows/    # CI validation and automated updates
```

---

## Notes for Upstream

This flake can easily be adopted by upstream:
- The `package.nix` derivation can be called directly on the upstream source tree via `pkgs.callPackage ./package.nix { src = ./.; }`.
- For `nixpkgs` inclusion, releases need to be permanently archived with versioned URLs (e.g. `edcas-client-source-157.tar.gz`) or Git release tags.