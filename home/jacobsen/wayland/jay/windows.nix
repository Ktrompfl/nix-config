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
      name = "ninjabrain-bot-xwayland";
      match = {
        app-id = "org.freedesktop.Xwayland";
        title = "Xwayland on :77";
        just-mapped = true;
      };
      auto-focus = false;
      initial-tile-state = "floating";
    }
  ];
}
