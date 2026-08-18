{ jayLib, ... }:
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
      name = "ninjabrain-bot-xwayland";
      match = {
        app-id = "org.freedesktop.Xwayland";
        # ninjabrain-bot-xwayland's default display; pass --display to move it.
        title = "Xwayland on :77";
        just-mapped = true;
      };
      auto-focus = false;
      initial-tile-state = "floating";
    }
  ];
}
