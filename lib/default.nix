{ lib, pkgs }:
{
  generators = lib.generators // import ./generators.nix { inherit lib; };
  jay = import ./jay.nix { inherit lib pkgs; };
}
