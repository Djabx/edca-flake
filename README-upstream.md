# EDCAS client — source

The source of the desktop client of [EDCAS](https://edcas.de), the Elite Dangerous
Commander Assistant System: the `edcas-client` crate and `edcas-common`, the shared
types it needs. The server and the web site are not part of this archive.

Build (Rust stable, see `crates/client/Cargo.toml` for the native libraries):

    cargo build --release -p edcas-client

The binary is `target/release/edcas-client`. `crates/client/README.md` is the client's
manual; `crates/client/shader_mods/` holds the shader mods the client installs into the
game (HLSL, the 3DMigoto ini and the ground-shader template), which the binary embeds.
Licence: see `LICENSE`.
