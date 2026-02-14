{ inputs, pkgs, ... }:
{
  home.packages = [
    inputs.hytale-launcher.packages.${pkgs.stdenv.hostPlatform.system}.default
  ];

  preservation.preserveAt.state-dir.directories = [
    ".local/share/hytale-launcher"
    ".local/share/Hytale"
  ];
}
