{ pkgs, inputs, ... }:
let
  inherit (pkgs) callPackage;
  jayScripts = callPackage ./jay-scripts { };
in
{
  inherit (jayScripts)
    jay-bar
    jay-clipboard-history
    jay-screenshot
    ;

  jay-config-lib = callPackage ./jay-config-lib { inherit inputs; };
  jay-session = callPackage ./jay-session.nix { };
  ninjabrain-box = callPackage ./ninjabrain-box { inherit inputs; };
  tinted-discord = callPackage ./tinted-discord { };
  tinted-zed = callPackage ./tinted-zed { };
  runic = callPackage ./runic.nix { };
  zed-julia = callPackage ./zed-julia.nix { inherit inputs; };
}
