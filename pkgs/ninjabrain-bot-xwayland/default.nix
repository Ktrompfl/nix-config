{
  imagemagick,
  lib,
  makeDesktopItem,
  makeWrapper,
  ninjabrain-bot,
  python3Packages,
  runCommand,
  wl-clipboard,
  writers,
  xclip,
  xwayland,
}:
let
  script = writers.writePython3Bin "ninjabrain-bot-xwayland" {
    libraries = [ python3Packages.xlib ];
    flakeIgnore = [ "E501" ];
  } (builtins.readFile ./main.py);

  desktopItem = makeDesktopItem {
    name = "ninjabrain-bot-xwayland";
    desktopName = "Ninjabrain Bot (Xwayland)";
    comment = "Stronghold calculator for Minecraft speedrunning";
    exec = "ninjabrain-bot-xwayland";
    icon = "ninjabrain-bot-xwayland";
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
in
runCommand "ninjabrain-bot-xwayland"
  {
    nativeBuildInputs = [
      imagemagick
      makeWrapper
    ];

    meta = {
      description = "Ninjabrain Bot (Xwayland)";
      mainProgram = "ninjabrain-bot-xwayland";
      inherit (ninjabrain-bot.meta) license;
    };
  }
  ''
    makeWrapper ${lib.getExe script} $out/bin/ninjabrain-bot-xwayland \
      --prefix PATH : ${
        lib.makeBinPath [
          ninjabrain-bot
          wl-clipboard
          xclip
          xwayland
        ]
      }

    install -Dm644 ${desktopItem}/share/applications/ninjabrain-bot-xwayland.desktop \
      $out/share/applications/ninjabrain-bot-xwayland.desktop

    # The bot ships one 640x640 icon, a size hicolor does not declare, so
    # nothing looking an icon up the documented way ever finds it.
    for size in 48 64 128 256 512; do
      dir=$out/share/icons/hicolor/''${size}x''${size}/apps
      mkdir -p $dir
      magick ${ninjabrain-bot}/share/icons/hicolor/640x640/apps/ninjabrain-bot.png \
        -resize ''${size}x''${size} $dir/ninjabrain-bot-xwayland.png
    done
  ''
