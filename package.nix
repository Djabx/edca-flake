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


}:

let
  src = lib.cleanSource ./. ;
  src3dmigoto = ./3dmigoto/edcas-client-3dmigoto-x64.zip;
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

  postPatch = lib.optionalString (src3dmigoto != null) ''
    substituteInPlace crates/client/src/shader_mods.rs \
      --replace-fail 'pub fn download(base_url: &str) -> Result<Bundle, String> {' \
        'pub fn download(base_url: &str) -> Result<Bundle, String> {
    if let Ok(path) = std::env::var("EDCAS_3DMIGOTO_ZIP") {
        if !path.is_empty() {
            let zip = fs::read(&path).map_err(|e| format!("cannot read 3DMigoto bundle from {path}: {e}"))?;
            return Ok(Bundle { zip });
        }
    }'
  '';

  postInstall = ''
    wrapProgram $out/bin/edcas-client \
      ${lib.optionalString (src3dmigoto != null) "--set EDCAS_3DMIGOTO_ZIP \"${src3dmigoto}\""} \
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
    substituteInPlace $out/share/applications/edcas-client.desktop \
      --replace-fail "Exec=/usr/bin/edcas-client" "Exec=edcas-client" \
      --replace-fail "Icon=/usr/share/edcas-client/edcas_128.png" "Icon=edcas"
    install -Dm644 assets/graphics/logo/edcas_128.png $out/share/icons/hicolor/128x128/apps/edcas.png
  '';

  meta = {
    description = "Elite Dangerous Commander Assistant System — desktop client";
    homepage = "https://edcas.de";
    license = lib.licenses.asl20;
    platforms = lib.platforms.linux;
    mainProgram = "edcas-client";
  };
}
