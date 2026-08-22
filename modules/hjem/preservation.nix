{ lib, ... }:
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
    '';
  };
}
