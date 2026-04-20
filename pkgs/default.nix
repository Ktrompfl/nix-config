{ inputs, pkgs, ... }:
{
  jay = pkgs.callPackage ./jay.nix { inherit inputs; };
  ninjabrainbot = pkgs.callPackage ./ninjabrainbot.nix { };
}
