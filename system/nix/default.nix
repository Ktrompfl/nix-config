{ pkgs, ... }:
{
  imports = [
    ./nh.nix
    ./nix-ld.nix
    ./optimise.nix
    ./registry.nix
    ./settings.nix
    ./substituters.nix
  ];

  nix.package = pkgs.lixPackageSets.stable.lix;
}
