{
  graphicalService,
  lib,
  pkgs,
  ...
}:
{
  # used to manually set the wallpaper
  packages = [ pkgs.awww ];

  systemd.services.awww = graphicalService "background" {
    description = "Animated wallpaper daemon for Wayland";
    path = [ pkgs.awww ];
    serviceConfig.ExecStart = lib.getExe' pkgs.awww "awww-daemon";
  };
}
