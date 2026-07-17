{ pkgs, inputs, ... }:
{
  jay-config-lib = pkgs.callPackage ./jay-config-lib { inherit inputs; };
  ninjabrainbot = pkgs.callPackage ./ninjabrainbot.nix { };
  zed-julia = pkgs.callPackage ./zed-julia.nix { inherit inputs; };
}
