{ pkgs, ... }:
{
  imports = [
    ./carrot
    ./jay

    ./fuzzel.nix
    ./i3status-rust.nix
    ./swaylock.nix
  ];

  packages = with pkgs; [
    wev # prints the wayland events a surface receives
    wl-clipboard
  ];
}
