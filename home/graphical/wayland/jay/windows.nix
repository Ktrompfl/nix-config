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
      # tile workspace containers on major axis
      match = {
        types = "container";
        is-workspace-container = true;
        just-mapped = true;
      };
      action = {
        type = "tile-major";
        target = "self";
      };
    }
  ];
}
