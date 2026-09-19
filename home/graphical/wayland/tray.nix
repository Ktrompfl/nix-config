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
    serviceConfig.ExecStart = lib.getExe pkgs.wl-tray-bridge;
  };
}
