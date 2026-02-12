{ inputs, pkgs, ... }:
{
  home.packages = [
    inputs.hytale-launcher.packages.${pkgs.stdenv.hostPlatform.system}.default
  ];
}
