{
  config,
  lib,
  ...
}:
{
  # The preservation system module allows to declare files and directories that should be persisted for a users home directory, e.g.
  # preservation.persistAt."/persist".users.jacobsen.directories = [ ".local" ];
  # This module allows to declare these files and directories directly in the home-manager user configuration.
  # note: The system module is still required, as files and directories are simply passed through to the system configuration.

  home-manager.sharedModules = [
    (
      let
        preserveAtSubmodule = {
          options = {
            directories = lib.mkOption {
              type = with lib.types; listOf (coercedTo str (d: { directory = d; }) attrs);
              default = [ ];
              description = ''
                Specify a list of directories that should be preserved for this user.
                The paths are interpreted relative to the user home directory.
              '';
            };
            files = lib.mkOption {
              type = with lib.types; listOf (coercedTo str (f: { file = f; }) attrs);
              default = [ ];
              description = ''
                Specify a list of files that should be preserved for this user.
                The paths are interpreted relative to the user home directory.
              '';
            };
          };
        };
      in
      {
        options.preservation.preserveAt = lib.mkOption {
          type =
            with lib.types;
            attrsWith {
              placeholder = "path";
              elemType = submodule preserveAtSubmodule;
            };
          description = ''
            Specify a set of locations and the corresponding state that
            should be preserved there.
          '';
          default = { };
        };
      }
    )
  ];

  # collect home-manager.users.<user>.preservation.preserveAt.<name> = <value>
  # to preservation.preserveAt.<name>.users.<user> = <value>
  preservation.preserveAt =
    let
      inherit (lib)
        foldl'
        mapAttrs
        mapAttrsToList
        recursiveUpdate
        ;
      hm-configs = config.home-manager.users or { };
      user-persist = mapAttrsToList (
        user: config:
        (mapAttrs (_: locations: {
          users.${user} = locations;
        }) (config.preservation.preserveAt or { }))
      ) hm-configs;
    in
    foldl' recursiveUpdate { } user-persist;
}
