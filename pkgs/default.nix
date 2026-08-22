{ pkgs, inputs, ... }:
let
  jayScripts = pkgs.callPackage ./jay-scripts {
    inherit (inputs.jay.packages.${pkgs.stdenv.hostPlatform.system}) jay;
  };
in
{
  tinted-discord = pkgs.callPackage ./tinted-discord { };
  tinted-zed = pkgs.callPackage ./tinted-zed { };

  inherit (jayScripts)
    jay-bar
    jay-clipboard-history
    jay-screenshot
    ;

  jay-config-lib = pkgs.callPackage ./jay-config-lib { inherit inputs; };
  ninjabrain-box = pkgs.callPackage ./ninjabrain-box { inherit inputs; };
  runic = pkgs.callPackage ./runic.nix { };
  zed-julia = pkgs.callPackage ./zed-julia.nix { inherit inputs; };
}
