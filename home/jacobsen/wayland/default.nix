{ pkgs, ... }:
{
  imports = [
    ./carrot
    ./jay
    ./swaync

    ./fuzzel.nix
    ./i3status-rust.nix
    ./swaylock.nix
  ];

  packages = with pkgs; [
    cliphist
    wl-clip-persist
    wl-clipboard

    wev # prints the wayland events a surface receives
  ];
}
