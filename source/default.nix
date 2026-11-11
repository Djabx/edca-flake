{
  lib,
  rustPlatform,
  pkg-config,
  makeWrapper,
  openssl,
  speechd,
  libxkbcommon,
  libglvnd,
  wayland,
  libx11,
  libxcursor,
  libxrandr,
  libxi,
  src,
}:

let
  cargoToml = builtins.fromTOML (builtins.readFile "${src}/crates/client/Cargo.toml");
in
rustPlatform.buildRustPackage {
  pname = "edcas-client";
  version = cargoToml.package.version;

  inherit src;

  cargoLock = {
    lockFile = "${src}/Cargo.lock";
  };

  nativeBuildInputs = [
    pkg-config
    makeWrapper
    rustPlatform.bindgenHook
  ];

  buildInputs = [
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

  postInstall = ''
    wrapProgram $out/bin/edcas-client \
      --prefix LD_LIBRARY_PATH : ${
        lib.makeLibraryPath [
          libglvnd
          libxkbcommon
          wayland
          libx11
          libxcursor
          libxrandr
          libxi
          speechd
        ]
      }

    install -Dm644 crates/client/packaging/edcas.desktop $out/share/applications/edcas-client.desktop
    install -Dm644 assets/graphics/logo/edcas_128.png $out/share/icons/hicolor/128x128/apps/edcas.png
  '';

  meta = {
    description = "Elite Dangerous Commander Assistant System — desktop client (built from source)";
    homepage = "https://edcas.de";
    license = lib.licenses.asl20;
    platforms = lib.platforms.linux;
    mainProgram = "edcas-client";
  };
}
