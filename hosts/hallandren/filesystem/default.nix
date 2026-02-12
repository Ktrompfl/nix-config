{ inputs, ... }:
{
  imports = [
    inputs.disko.nixosModules.disko

    ./disks.nix
    ./zfs.nix
    ./zpool.nix
  ];
}
