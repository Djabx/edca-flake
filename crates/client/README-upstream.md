# edcas-client

The desktop client for [EDCAS](https://edcas.de) — *Elite Dangerous Commander
Assistant System*. A tabbed window styled after the in-game HUD (built with
[`egui`](https://www.egui.rs) on OpenGL) that reads your Elite Dangerous journal
logs live and organises everything into scannable views, plus galaxy/trade
features powered by the edcas server.

The window draws only when something changes (input, a journal event, a finished
search, once a second for clocks), and uses OpenGL rather than wgpu/Vulkan, so it
stays out of the game's way: about 20 MB of VRAM and well under 1 ms of GPU time
per second while idle. Most things can be done from the keyboard too; `?` shows
the shortcuts.

This crate produces the `edcas-client` binary (and a library of the same name).
This README is the full client manual.

## Features

- **Exploration** — compact system/body view, terraform state, estimated
  scan/mapping value, gravity, atmosphere, materials, signals, and a mining tab.
- **Commander & ship** — ranks/reputation/credits/powerplay, ship & on-foot
  loadout, and an engineering workshop (materials, modules, blueprint progress).
- **Galaxy & trade** (via the edcas server / EDDN) — station & fleet-carrier
  search with live market/outfitting/shipyard, faction/BGS search, trade-route and
  trade-loop finders, "nearest commodity", construction-depot tracking, GalNet news.
- **Quality of life** — live background journal watcher, pins + personal todo list,
  on-screen alert toasts (optional spoken voice), runs on Linux/Windows/macOS.
- **Data sharing** — uploads a sanitised subset to EDDN and, optionally, the edcas
  API (each an independent opt-in toggle).

## How it's built

| Path | Purpose |
|---|---|
| `src/main.rs` | Entry point: CLI flags (`--upload-logs`, `--headless`, `--version`), window startup |
| `src/gui/` | The window: shell (tabs, status bar, toasts, help), HUD theme + fonts, widget kit |
| `src/app.rs`, `src/views/` | App state and per-tab input logic; each view draws itself in `views/<view>/gui.rs` |
| `src/journal_reader.rs` | Background journal file watcher (UI updates live) |
| `src/eddn.rs` | Journal → EDDN-schema conversion and upload |
| `src/api_client.rs` | Async client for the edcas REST API |
| `src/alerts.rs`, `src/tts.rs` | Alert detection and spoken-alert engine |
| `src/settings.rs`, `settings-example.json` | Config model + embedded default template |
| `src/build_info.rs` | `BUILD` const stamped from `EDCAS_BUILD` at compile time |
| `packaging/` | `edcas.desktop` and the Arch `PKGBUILD` (`.deb` metadata is in `Cargo.toml`) |

Fonts: Orbitron and Rajdhani (SIL Open Font License 1.1) for the HUD look, DejaVu Sans
for symbols; they are embedded in the binary, licences in `assets/fonts/`.

The build number (`build_info::BUILD`, from the CI pipeline) shows in the status
bar and `--version`; at startup the client compares it to the server's
`/api/v1/client-version` and flags itself **outdated** on a mismatch.

## Install

**Pre-built packages** — from the
[releases page](https://github.com/arne-fuchs/edcas-client/releases) or
<https://edcas.de> → *Client* (Linux binary, Windows `.exe`, `.deb`, Arch
`.pkg.tar.zst`):

```bash
sudo dpkg -i edcas-client*.deb        # Debian / Ubuntu
makepkg -i                             # Arch (with the PKGBUILD)
```

**Build from source** — needs Rust (stable). Linux build dependencies:

```bash
sudo apt install cmake pkg-config build-essential git \
    libwayland-dev libglib2.0-dev libxdo-dev \
    libspeechd-dev clang libclang-dev
# libspeechd-dev + clang/libclang-dev are for the optional spoken alerts
# (the speech-dispatcher binding generates code with bindgen, which needs libclang)
cargo run --release -p edcas-client
```

`EDCAS_BUILD=<n> cargo build --release -p edcas-client` stamps a build/version
number. Packages are produced by the CI pipeline with the pipeline id as the version.

### Configuration

| File | Default location | Fallback |
|---|---|---|
| `settings.json` | `$HOME/.config/edcas-client/settings.json` | `./settings.json` |
| Assets (`edcas_128.png`, …) | `/usr/share/edcas-client/` | `./` |

On first run, config is seeded from an embedded default template (an admin-installed
`/etc/edcas-client/settings-example.json` overrides it if present). To launch
alongside the game, add `/usr/bin/edcas-client` to your
[min-ed-launcher](https://github.com/rfvgyhn/min-ed-launcher) `processes`.

## Data uploads (EDDN & edcas API)

Both destinations are **on by default**; each toggles independently in **Settings**.

| Setting | Default | Description |
|---|---|---|
| `edcas_api_enabled` | `true` | Upload journal events to the edcas API (`api_url`) for search/trade features |
| `eddn_enabled` | `true` | Upload to the public [EDDN](https://github.com/EDCD/EDDN) network |
| `eddn_test_mode` | `true` | Send to EDDN's **test** pipeline (validated, not relayed) |

Like EDMC/EDDiscovery, the client converts a curated, **sanitised** subset of the
journal into EDDN schemas (`journal/1`, `commodity/3`, `outfitting/2`,
`shipyard/2`); `_Localised` strings and Cmdr-specific fields are stripped. It
identifies as `softwareName` `EDCAS` with a `0.4.0+<build>` version. There's no
EDDN registration — turn off **EDDN Test Mode** once you've confirmed `HTTP 200`
uploads to contribute to the live network.

## Alerts & voice

Alerts fire once when a condition first occurs and re-arm only after it clears.
Configured under **Settings → Alerts** (persisted in `settings.json`):

| Group | Alerts |
|---|---|
| Hazards | Low Fuel, Overheating, Being Interdicted, In Danger, Low Oxygen, Low Health |
| Discoveries | Non-Human (Thargoid) Signal, Guardian Signal, High Bio Signals, High Geo Signals |
| Fleet Carrier | Carrier Jump Ready |

Keys: `enabled`, `voice`, `voice_volume` (0–100), `high_bio_threshold`,
`high_geo_threshold`, plus a per-alert toggle each.

**Voice** uses the OS speech engine via the [`tts`](https://crates.io/crates/tts)
crate (no bundled binaries): Windows SAPI 5, macOS AVFoundation, Linux Speech
Dispatcher (needs `speech-dispatcher` running + an output module like `espeak-ng`;
`libspeechd-dev` to build). If none is available the feature stays silent. For
much more natural Linux voices, see
[improving speech on Linux](../../docs/voice-on-linux.md).
