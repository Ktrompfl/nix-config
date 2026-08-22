{ pkgs, ... }:
{
  imports = [
    ./programs/minecraft

    ./programs/mangohud.nix
    ./programs/moonlight.nix
    ./programs/steam.nix
  ];

  packages = [ pkgs.gpu-screen-recorder-gtk ];
}
