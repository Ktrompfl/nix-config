{ inputs, ... }:
{
  imports = [
    inputs.self.nixosModules.default
    inputs.carrot.nixosModules.default
    inputs.jay.nixosModules.default

    ./boot
    ./hardware
    ./network
    ./nix
    ./programs
    ./services

    ./theme
    ./localization.nix
    ./packages.nix
    ./oomd.nix
    ./preservation.nix
    ./security.nix
    ./sops.nix
    ./users.nix
  ];

  documentation = {
    doc.enable = false;
    info.enable = false;
    nixos.enable = false;
  };

  nixpkgs = {
    config.allowUnfree = true;
    overlays = [ inputs.self.overlays.default ];
  };
}
