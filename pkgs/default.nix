{ pkgs, inputs, ... }:
let
  jayScripts = pkgs.callPackage ./jay-scripts { };
in
{
  inherit (jayScripts)
    jay-bar
    jay-clipboard-history
    jay-screenshot
    jay-suspend
    ;

  jay-config-lib = pkgs.callPackage ./jay-config-lib { inherit inputs; };
  ninjabrainbot = pkgs.callPackage ./ninjabrainbot.nix { };
  runic = pkgs.callPackage ./runic.nix { };
  zed-julia = pkgs.callPackage ./zed-julia.nix { inherit inputs; };
}
