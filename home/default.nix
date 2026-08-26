{
  config,
  inputs,
  lib,
  ...
}:
{
  imports = [ inputs.hjem.nixosModules.default ];

  hjem = {
    clobberByDefault = false;

    specialArgs = { inherit inputs; };

    extraModules = [ inputs.self.hjemModules.default ];

    users.jacobsen = {
      enable = true;
      user = "jacobsen";
      directory = "/home/jacobsen";

      imports = [ ./jacobsen ];
    };
  };

  # `systemd --user` has no mount units, so every user's own `systemd.mounts` is
  # folded into the system manager's.
  systemd.mounts = lib.concatMap (userConfig: userConfig.systemd.mounts or [ ]) (
    lib.attrValues config.hjem.users
  );

  # every user's own `preservation.preserveAt` is folded into the system-wide option below the `users.<name>` key the preservation module expects it under.
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
