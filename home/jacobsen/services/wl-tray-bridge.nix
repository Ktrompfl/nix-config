{
  sandboxedService,
  lib,
  pkgs,
  ...
}:
{
  packages = [ pkgs.wl-tray-bridge ];

  systemd.services.wl-tray-bridge = sandboxedService "session" {
    description = "Bridge between StatusNotifierItem tray applications and jay's tray protocol";
    serviceConfig.ExecStart = lib.getExe pkgs.wl-tray-bridge;
  };
}
