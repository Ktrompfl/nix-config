{ pkgs, ... }:
{
  ninjabrainbot = pkgs.callPackage ./ninjabrainbot.nix { };
}
