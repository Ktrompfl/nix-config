# Starts Ninjabrain Bot, in a box or bare, and sends it its own hotkeys; see
# ./nbb_ootb.py. Everything it needs is baked into one JSON file, so it depends
# on nothing in the environment it is exec'd from.
{
  bash,
  lib,
  makeDesktopItem,
  ninjabrain-bot,
  python3Packages,
  symlinkJoin,
  wl-clipboard,
  writeText,
  writers,
  xclip,
  xwayland,

  # Filled in by the home-manager module. Without prefs the bot's own settings
  # file is left alone, so the bare package still works.
  prefs ? null,
  actions ? { },
  display ? ":77",
  geometry ? "480x320",
  prefsPath ? "~/.java/.userPrefs/ninjabrainbot/prefs.xml",
}:
let
  config = writeText "nbb-ootb.json" (
    builtins.toJSON {
      inherit
        actions
        display
        geometry
        prefs
        prefsPath
        ;
      bot = lib.getExe ninjabrain-bot;
      xwayland = lib.getExe' xwayland "Xwayland";
      wlPaste = lib.getExe' wl-clipboard "wl-paste";
      xclip = lib.getExe xclip;
      # Only ever runs the one-liner that delimits clipboard entries.
      shell = lib.getExe' bash "bash";
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

  # The bot's own icon, which is not otherwise installed: only this wrapper ends
  # up in the profile.
  postBuild = ''
    mkdir -p $out/share
    cp -r ${ninjabrain-bot}/share/icons $out/share/
  '';

  meta = {
    description = "Ninjabrain Bot, wrapped so that it works under Wayland";
    mainProgram = "nbb-ootb";
    inherit (ninjabrain-bot.meta) license;
  };
}
