{
  config,
  inputs,
  lib,
  ...
}:
{
  imports = [
    inputs.hjem.nixosModules.default
    ./services
  ];

  hjem = {
    clobberByDefault = false;

    specialArgs = { inherit inputs; };

    extraModules = [
      inputs.self.hjemModules
      ./preservation.nix
    ];

    users.jacobsen = {
      enable = true;
      user = "jacobsen";
      directory = "/home/jacobsen";

      imports = [
        ./desktop
        ./programs
        ./wayland

        ./environment.nix
        ./packages.nix
      ];
    };
  };

  # ../preservation.nix, for the user scope.
  preservation.preserveAt =
    let
      inherit (lib)
        foldl'
        mapAttrs
        mapAttrsToList
        recursiveUpdate
        ;
    in
    foldl' recursiveUpdate { } (
      mapAttrsToList (
        user: userConfig:
        mapAttrs (_: locations: { users.${user} = locations; }) (userConfig.preservation.preserveAt or { })
      ) config.hjem.users
    );
}
