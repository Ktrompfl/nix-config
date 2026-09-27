{
  config,
  lib,
  osConfig,
  ...
}:
let
  preserveAtSubmodule = {
    options = {
      directories = lib.mkOption {
        type = with lib.types; listOf (coercedTo str (d: { directory = d; }) attrs);
        default = [ ];
        description = ''
          Directories to preserve for this user, interpreted relative to the
          user's home directory.
        '';
      };

      files = lib.mkOption {
        type = with lib.types; listOf (coercedTo str (f: { file = f; }) attrs);
        default = [ ];
        description = ''
          Files to preserve for this user, interpreted relative to the user's
          home directory.
        '';
      };
    };
  };

  storageOf = location: "${osConfig.preservation.preserveAt.${location}.persistentStoragePath}${config.directory}";
in
{
  options.preservation.preserveAt = lib.mkOption {
    type =
      with lib.types;
      attrsWith {
        placeholder = "path";
        elemType = submodule preserveAtSubmodule;
      };
    default = { };
    description = ''
      Locations and the corresponding state that should be preserved there.
      Only for programs that cannot be pointed at `storage` directly.
    '';
  };

  options.storage = {
    data = lib.mkOption {
      type = lib.types.str;
      readOnly = true;
      default = storageOf "data-dir";
      description = ''
        This user's part of the backed up partition. State that cannot be
        recreated goes below it, addressed by this path rather than through a
        link in the home directory.
      '';
    };

    state = lib.mkOption {
      type = lib.types.str;
      readOnly = true;
      default = storageOf "state-dir";
      description = ''
        This user's part of the machine-local partition, for state that is
        worth keeping across reboots but not worth backing up. The xdg data,
        state and cache homes are below it.
      '';
    };
  };
}
