{ pkgs, inputs, ... }:
{
  ninjabrainbot = pkgs.callPackage ./ninjabrainbot.nix { };
  zed-julia = pkgs.callPackage ./zed-julia.nix { inherit inputs; };
}
