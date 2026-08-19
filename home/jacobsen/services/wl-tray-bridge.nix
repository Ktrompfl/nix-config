{
  graphicalService,
  lib,
  pkgs,
  ...
}:
{
  # tray icons vanish for every app that has one
  systemd.user.services.wl-tray-bridge = graphicalService "session" {
    description = "Bridge between StatusNotifierItem tray applications and jay's tray protocol";
    serviceConfig.ExecStart = lib.getExe pkgs.wl-tray-bridge;
  };
}
