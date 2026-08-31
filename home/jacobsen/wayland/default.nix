{ pkgs, ... }:
{
  imports = [
    ./jay

    ./fuzzel.nix
    ./i3status-rust.nix
    ./swaylock.nix
  ];

  environment.sessionVariables = {
    NIXOS_OZONE_WL = "1";
    # TODO: check what these are still required for
    # CLUTTER_BACKEND = "wayland";
    # SDL_VIDEODRIVER = "wayland";
  };

  packages = with pkgs; [
    wev # prints the wayland events a surface receives
    wl-clipboard
  ];
}
