{
  config,
  lib,
  pkgs,
  ...
}:
{
  home.packages = [ pkgs.sh-calc ];

  systemd.user.services.sh-calc = {
    Unit = {
      Description = "Minecraft stronghold calculator for 1.16 boat eye";
      PartOf = [ config.wayland.systemd.target ];
      After = [ config.wayland.systemd.target ];
    };
    Service = {
      ExecStart = "${lib.getExe pkgs.sh-calc} daemon";
      Restart = "on-failure";
      RestartSec = 5;
    };
  };
}
