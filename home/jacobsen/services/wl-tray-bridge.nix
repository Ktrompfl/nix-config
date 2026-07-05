{
  config,
  lib,
  pkgs,
  ...
}:
{
  systemd.user.services.wl-tray-bridge = {
    Unit = {
      Description = "Bridge between StatusNotifierItem tray applications and jay's tray protocol";
      PartOf = [ config.wayland.systemd.target ];
      After = [ config.wayland.systemd.target ];
    };
    Service = {
      ExecStart = lib.getExe pkgs.wl-tray-bridge;
      Restart = "on-failure";
    };
    Install.WantedBy = [ config.wayland.systemd.target ];
  };
}
