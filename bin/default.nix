{
  lib,
  stdenv,
  autoPatchelfHook,
  makeWrapper,
  speechd,
  libxkbcommon,
  libglvnd,
  wayland,
  libx11,
  libxcursor,
  libxrandr,
  libxi,
  openssl,
  gcc-unwrapped,
  srcBinary,
  srcAssets ? null,
}:

let
  version =
    if srcAssets != null then
      (builtins.fromTOML (builtins.readFile "${srcAssets}/crates/client/Cargo.toml")).package.version
    else
      "latest";
in
stdenv.mkDerivation {
  pname = "edcas-client-bin";
  inherit version;

  src = srcBinary;
  dontUnpack = true;

  nativeBuildInputs = [
    autoPatchelfHook
    makeWrapper
  ];

  buildInputs = [
    speechd
    libxkbcommon
    libglvnd
    wayland
    libx11
    libxcursor
    libxrandr
    libxi
    openssl
    gcc-unwrapped.lib
  ];

  installPhase = ''
    runHook preInstall

    install -Dm755 $src $out/bin/edcas-client

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

    ${lib.optionalString (srcAssets != null) ''
      install -Dm644 ${srcAssets}/crates/client/packaging/edcas.desktop $out/share/applications/edcas-client.desktop
      install -Dm644 ${srcAssets}/assets/graphics/logo/edcas_128.png $out/share/icons/hicolor/128x128/apps/edcas.png
    ''}

    runHook postInstall
  '';

  meta = {
    description = "Elite Dangerous Commander Assistant System — desktop client (pre-built binary)";
    homepage = "https://edcas.de";
    license = lib.licenses.asl20;
    platforms = [ "x86_64-linux" ];
    mainProgram = "edcas-client";
  };
}
