{ config, jayLib, ... }:
let
  inherit (jayLib) moveToOutput moveToWorkspace;
in
{
  # What happens to a window when it is mapped.
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
      name = "nbb-ootb";
      match = {
        app-id = "org.freedesktop.Xwayland";
        title = "Xwayland on ${config.programs.ninjabrain-bot.display}";
        just-mapped = true;
      };
      auto-focus = false;
      initial-tile-state = "floating";
    }
  ];
}
