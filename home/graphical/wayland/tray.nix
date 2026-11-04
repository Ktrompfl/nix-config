{
  lib,
  pkgs,
  ...
}:
{
  packages = [ pkgs.wl-tray-bridge ];

  wayland.services.wl-tray-bridge = {
    slice = "session";

    description = "Bridge between StatusNotifierItem tray applications and jay's tray protocol";
    # without --config it writes its defaults to ~/.config
    serviceConfig.ExecStart = "${lib.getExe pkgs.wl-tray-bridge} --config ${
      (pkgs.formats.toml { }).generate "wl-tray-bridge.toml" { }
    }";
  };
}
