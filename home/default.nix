{ inputs, ... }:
{
  imports = [
    inputs.home-manager.nixosModules.home-manager

    ./preservation.nix
  ];

  home-manager = {
    extraSpecialArgs = { inherit inputs; };
    useGlobalPkgs = true;
    useUserPackages = true;
    backupFileExtension = "hm-bak";

    sharedModules = [
      # Import all modules this flake exports (from modules/home-manager):
      inputs.self.homeManagerModules
    ];

    # user configurations
    users.jacobsen = import ./jacobsen;
  };
}
