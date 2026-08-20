{ jayLib, ... }:
let
  inherit (jayLib) moveToOutput moveToWorkspace;
in
{
  windows = [
    {
      name = "wl-mirror";
      match = {
        app-id = "at.yrlf.wl_mirror";
        just-mapped = true;
      };
      auto-focus = false;
      action = [
        (moveToWorkspace "0")
        (moveToOutput {
          workspace = "0";
          output.name = "beamer";
        })
        "enter-fullscreen"
      ];
    }
    {
      name = "ninjabrain-bot";
      match.x-class = "ninjabrainbot-Main";
      auto-focus = false;
      initial-tile-state = "floating";
    }
  ];
}
