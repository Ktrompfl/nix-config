{ pkgs, inputs, ... }:
let
  inherit (pkgs) callPackage;
in
{
  firefox-search = callPackage ./firefox-search.nix { };
  getaway = callPackage ./getaway { inherit inputs; };
  i3bar-wl = callPackage ./i3bar-wl { inherit inputs; };
  jay-config-lib = callPackage ./jay-config-lib { inherit inputs; };
  ninjabrain-box = callPackage ./ninjabrain-box { inherit inputs; };
  tinted-discord = callPackage ./tinted-discord { };
  tinted-vscode = callPackage ./tinted-vscode { };
  tinted-zed = callPackage ./tinted-zed { };
}
