{ lib, ... }:
{
  # installed by default in graphical-desktop.nix
  services.speechd.enable = lib.mkForce false;
}
