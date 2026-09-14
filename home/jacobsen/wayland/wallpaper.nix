{
  config,
  lib,
  pkgs,
  ...
}:
{
  # used to manually set the wallpaper
  packages = [ pkgs.awww ];

  wayland.services.awww = {
    description = "Animated wallpaper daemon for Wayland";
    path = [ pkgs.awww ];

    serviceConfig = {
      ExecStart = lib.getExe' pkgs.awww "awww-daemon";

      BindReadOnlyPaths = [ "${config.directory}/Pictures" ];
      BindPaths = [ "${config.directory}/.local/cache" ];
    };
  };
}
