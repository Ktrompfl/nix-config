# local package until https://github.com/NixOS/nixpkgs/pull/342187 is merged
{
  lib,
  stdenv,
  fetchurl,
  makeWrapper,
  makeDesktopItem,
  copyDesktopItems,
  jre,
  libxkbcommon,
  libx11,
  libxt,
  libxtst,
  libxext,
  libxi,
  libxrender,
  libxrandr,
  libxfixes,
  libxkbfile,
  xkeyboard_config,
}:
stdenv.mkDerivation rec {
  name = "ninjabrainbot";
  version = "1.5.1";

  src = fetchurl {
    url = "https://github.com/Ninjabrain1/Ninjabrain-Bot/releases/download/${version}/Ninjabrain-Bot-${version}.jar";
    sha256 = "sha256-Rxu9A2EiTr69fLBUImRv+RLC2LmosawIDyDPIaRcrdw=";
  };

  dontUnpack = true;

  nativeBuildInputs = [
    makeWrapper
    copyDesktopItems
  ];

  # All required runtime libraries for the application to function correctly.
  runtimeLibs = [
    libxkbcommon
    libx11
    libxt
    libxtst
    libxext
    libxi
    libxrender
    libxrandr
    libxfixes
    libxkbfile
    xkeyboard_config
  ];

  # jre and Xwayland are also build inputs.
  buildInputs = [ jre ] ++ runtimeLibs;

  installPhase = ''
    runHook preInstall
    mkdir -p $out/bin $out/share/java
    cp $src $out/share/java/ninjabrain-bot.jar

    # This wrapper sets the necessary library path for all dependencies
    # and a required AWT flag for compatibility with modern window managers.
    makeWrapper ${jre}/bin/java $out/bin/ninjabrain-bot \
      --add-flags "-Dswing.defaultlaf=javax.swing.plaf.metal.MetalLookAndFeel -Dawt.useSystemAAFontSettings=on -Dswing.aatext=true -jar $out/share/java/ninjabrain-bot.jar" \
      --prefix LD_LIBRARY_PATH : ${lib.makeLibraryPath runtimeLibs}
    runHook postInstall
  '';

  desktopItems = [
    (makeDesktopItem {
      name = "ninjabrainbot";
      desktopName = "Ninjabrain-Bot";
      exec = "ninjabrain-bot";
      startupWMClass = "Ninjabrain-Bot";
      genericName = "Stronghold Calculator";
      comment = "Accurate stronghold calculator for Minecraft speedrunning";
      keywords = [
        "minecraft"
        "speedrun"
        "ninjabrain"
        "stronghold"
        "calculator"
      ];
      categories = [
        "Utility"
        "Calculator"
        "Java"
      ];
    })
  ];

  meta = {
    homepage = "https://github.com/Ninjabrain1/Ninjabrain-Bot";
    description = "Accurate stronghold calculator for Minecraft speedrunning.";
    mainProgram = "ninjabrain-bot";
    platforms = lib.platforms.linux;
    sourceProvenance = [ lib.sourceTypes.binaryBytecode ];
    license = lib.licenses.unfree; # sadly this package has no license
  };
}
