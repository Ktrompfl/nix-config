{
  dejavu_fonts,
  imagemagick,
  inputs,
  lib,
  makeDesktopItem,
  makeWrapper,
  ninjabrain-bot,
  pkgs,
  xorgserver,
}:
let
  desktopItem = makeDesktopItem {
    name = "ninjabrain-box";
    desktopName = "Ninjabrain Box";
    comment = "Stronghold calculator for Minecraft speedrunning, as an overlay";
    exec = "ninjabrain-box";
    icon = "ninjabrain-box";
    categories = [
      "Game"
      "Utility"
    ];
    keywords = [
      "minecraft"
      "speedrun"
      "stronghold"
      "mcsr"
    ];
  };

  craneLib = inputs.crane.mkLib pkgs;

  commonArgs = {
    src = lib.fileset.toSource {
      root = ./.;
      fileset = lib.fileset.unions [
        ./Cargo.toml
        ./Cargo.lock
        ./src
      ];
    };
    pname = "ninjabrain-box";
    version = "0.1.0";
    strictDeps = true;
    doCheck = false;
  };

  cargoArtifacts = craneLib.buildDepsOnly commonArgs;
in
craneLib.buildPackage (
  commonArgs
  // {
    inherit cargoArtifacts;

    nativeBuildInputs = [
      imagemagick
      makeWrapper
    ];

    # The bot is the published release, run as published. Nothing here patches
    # it, injects into it or wraps its launcher: it is handed a display of its
    # own and a preferences file, and everything else it does, it does itself.
    postInstall = ''
      wrapProgram $out/bin/ninjabrain-box \
        --set NINJABRAIN_BOX_BOT ${lib.getExe ninjabrain-bot} \
        --prefix PATH : ${lib.makeBinPath [ xorgserver ]} \
        --set-default NINJABRAIN_BOX_FONT \
          ${dejavu_fonts}/share/fonts/truetype/DejaVuSansMono.ttf

      install -Dm644 ${desktopItem}/share/applications/ninjabrain-box.desktop \
        $out/share/applications/ninjabrain-box.desktop

      # The bot ships one 640x640 icon, a size hicolor does not declare, so
      # nothing looking an icon up the documented way ever finds it.
      icon=${ninjabrain-bot}/share/icons/hicolor/640x640/apps/ninjabrain-bot.png
      install -Dm644 $icon $out/share/icons/hicolor/640x640/apps/ninjabrain-box.png
      for size in 16 24 32 48 64 128 256 512; do
        dir=$out/share/icons/hicolor/''${size}x''${size}/apps
        mkdir -p $dir
        magick $icon -resize ''${size}x''${size} $dir/ninjabrain-box.png
      done
    '';

    meta = {
      description = "Ninjabrain Bot on a display of its own, with a Wayland overlay for its panel";
      mainProgram = "ninjabrain-box";
      inherit (ninjabrain-bot.meta) license;
      platforms = lib.platforms.linux;
    };
  }
)
