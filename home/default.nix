{
  config,
  inputs,
  lib,
  ...
}:
let
  inherit (lib)
    attrValues
    concatMap
    foldl'
    mapAttrs
    mapAttrsToList
    recursiveUpdate
    ;

  # symlink instead of bindmount by default; create directory targets
  preservationDefaults = {
    directories = {
      how = "symlink";
      createLinkTarget = true;
    };
    files.how = "symlink";
  };

  withDefaults = mapAttrs (kind: map (entry: preservationDefaults.${kind} // entry));

  # a user's `preserveAt.<location>` belongs under the `users.<name>` key that
  # the preservation module expects per-user state beneath.
  preservationOf =
    user: userConfig:
    mapAttrs (_: locations: { users.${user} = withDefaults locations; }) (
      userConfig.preservation.preserveAt or { }
    );
in
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
  systemd.mounts = concatMap (userConfig: userConfig.systemd.mounts or [ ]) (
    attrValues config.hjem.users
  );

  # every user's own `preservation.preserveAt` is folded into the system-wide one.
  preservation.preserveAt = foldl' recursiveUpdate { } (
    mapAttrsToList preservationOf config.hjem.users
  );
}
