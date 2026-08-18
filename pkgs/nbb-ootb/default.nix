# Starts Ninjabrain Bot in an X server of its own and sends it its own hotkeys;
# see ./nbb_ootb.py. The tools it drives come in on PATH, its settings in one
# JSON file, so it depends on nothing in the environment it is exec'd from.
{
  lib,
  makeDesktopItem,
  makeWrapper,
  ninjabrain-bot,
  python3Packages,
  symlinkJoin,
  wl-clipboard,
  writeText,
  writers,
  xclip,
  xwayland,

  # Filled in by the home-manager module.
  actions ? { },
  display ? ":77",
  geometry ? "480x320",
}:
let
  config = writeText "nbb-ootb.json" (
    builtins.toJSON {
      inherit actions display geometry;
    }
  );

  script = writers.writePython3Bin "nbb-ootb" {
    libraries = [ python3Packages.xlib ];
    flakeIgnore = [ "E501" ];
  } (builtins.replaceStrings [ "@config@" ] [ "${config}" ] (builtins.readFile ./nbb_ootb.py));

  desktopItem = makeDesktopItem {
    name = "nbb-ootb";
    desktopName = "Ninjabrain Bot";
    comment = "Stronghold calculator for Minecraft speedrunning";
    exec = "nbb-ootb";
    icon = "ninjabrain-bot";
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
symlinkJoin {
  name = "nbb-ootb";
  paths = [
    script
    desktopItem
  ];

  nativeBuildInputs = [ makeWrapper ];

  # The bot's own icon, which is not otherwise installed: only this wrapper ends
  # up in the profile.
  postBuild = ''
    rm $out/bin/nbb-ootb
    makeWrapper ${lib.getExe script} $out/bin/nbb-ootb \
      --prefix PATH : ${
        lib.makeBinPath [
          ninjabrain-bot
          wl-clipboard
          xclip
          xwayland
        ]
      }

    mkdir -p $out/share
    cp -r ${ninjabrain-bot}/share/icons $out/share/
  '';

  meta = {
    description = "Ninjabrain Bot, wrapped so that it works under Wayland";
    mainProgram = "nbb-ootb";
    inherit (ninjabrain-bot.meta) license;
  };
}
