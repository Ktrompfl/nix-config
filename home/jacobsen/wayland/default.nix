{ pkgs, ... }:
{
  imports = [
    ./jay
    ./notifications

    ./clipboard.nix
    ./idle.nix
    ./launcher.nix
    ./service.nix
    ./status.nix
    ./terminal.nix
    ./tray.nix
    ./wallpaper.nix
  ];

  environment.sessionVariables = {
    NIXOS_OZONE_WL = "1";
    # TODO: check what these are still required for
    # CLUTTER_BACKEND = "wayland";
    # SDL_VIDEODRIVER = "wayland";
  };

  packages = with pkgs; [
    wev # prints the wayland events a surface receives
  ];
}
