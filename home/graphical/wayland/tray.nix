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
    # Without `--config` it writes its defaults to ~/.config first, which the
    # sandbox's read-only home refuses; an empty file means the defaults.
    serviceConfig.ExecStart = "${lib.getExe pkgs.wl-tray-bridge} --config ${
      (pkgs.formats.toml { }).generate "wl-tray-bridge.toml" { }
    }";
  };
}
