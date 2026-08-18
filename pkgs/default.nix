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
  ninjabrain-bot-xwayland = pkgs.callPackage ./ninjabrain-bot-xwayland { };
  runic = pkgs.callPackage ./runic.nix { };
  zed-julia = pkgs.callPackage ./zed-julia.nix { inherit inputs; };
}
