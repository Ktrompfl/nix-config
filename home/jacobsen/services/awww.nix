{
  graphicalService,
  lib,
  pkgs,
  ...
}:
{
  # used to manually set the wallpaper
  hjem.users.jacobsen.packages = [ pkgs.awww ];

  # the wallpaper; losing it is cosmetic
  systemd.user.services.awww = graphicalService "background" {
    description = "Animated wallpaper daemon for Wayland";
    serviceConfig.ExecStart = lib.getExe' pkgs.awww "awww-daemon";
  };
}
